use super::*;
use std::io::Write;

pub(super) async fn execute(spec: TaskSpec, resume_id: Option<&str>) -> Result<ExitCode, CliError> {
    let endpoint = url::Url::parse(&spec.endpoint)
        .map_err(|_| fail("invalid_model_endpoint", "模型入口格式錯誤"))?;
    let loopback = match endpoint.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    if !loopback
        || !matches!(endpoint.scheme(), "http" | "https")
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
        || spec.model.trim().is_empty()
    {
        return Err(fail(
            "invalid_model_endpoint",
            "目前只接受明確的本機 IP 模型入口與非空模型名稱；網址不得夾帶憑證或查詢參數",
        ));
    }
    let mode = if resume_id.is_some() {
        ApprovalMode::Manual
    } else {
        spec.approval_mode
    };
    let manual = mode == ApprovalMode::Manual;
    let key = match std::env::var("AGENT_API_KEY") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => String::new(),
        Err(_) => return Err(fail("invalid_api_key", "AGENT_API_KEY 格式錯誤")),
    };
    let model = OpenAi::new(&endpoint, spec.model.clone(), key, task_timeout(mode))
        .map_err(|error| fail("model_client_failed", &error.0))?;
    let cancellation = CancellationToken::new();
    watch_control::watch_control(cancellation.clone())?;
    let signal_cancellation = cancellation.clone();
    let mut interrupted = false;
    ctrlc::try_set_handler(move || {
        if interrupted {
            std::process::exit(130);
        }
        interrupted = true;
        signal_cancellation.cancel();
    })
    .map_err(|_| fail("signal_handler_failed", "無法接收停止要求，任務未啟動"))?;
    let mut opened = open_store(StoreAccess::Create)?;
    let task_id = match resume_id {
        Some(id) => id.to_owned(),
        None => opened.start_spec(&spec)?.0,
    };
    let store = Arc::new(Mutex::new(opened));
    let (session, artifact) = if resume_id.is_some() {
        agent_platform_agent::task_session::TaskSession::resume(
            store.clone(),
            task_id.clone(),
            &spec,
        )?
    } else {
        let generation = store
            .lock()
            .map_err(|_| fail("store_state_poisoned", "任務紀錄狀態異常"))?
            .status(&task_id)?
            .generation;
        let session = agent_platform_agent::task_session::TaskSession::new(
            store.clone(),
            task_id.clone(),
            generation,
        )
        .map_err(|_| fail("session_unavailable", "無法取得任務執行權，任務未派工"))?;
        (session, None)
    };
    let generation = session.generation();
    #[cfg(target_os = "linux")]
    session.register_process()?;
    let mut watcher = crate::task_cancellation::TaskCancellation::start(
        task_id.clone(),
        generation,
        cancellation.clone(),
    )?;
    let gate = task_gate(
        store.clone(),
        task_id.clone(),
        generation,
        cancellation.clone(),
        manual,
    );
    let announced = crate::announce_task::announce_task(&task_id, manual);
    if announced.is_err() {
        cancellation.cancel();
    }
    let mut result = run_file_task_with_session(
        model,
        spec.task,
        cancellation,
        gate.clone(),
        task_timeout(mode),
        &session,
        artifact,
    )
    .await;
    if let Err(error) = watcher.stop() {
        eprintln!("{}", error.line());
        result = Err(TaskError::State);
    }
    if matches!(result, Err(TaskError::Harness(_))) && gate.was_denied() {
        result = Err(TaskError::ApprovalDenied);
    }
    let mut store = store
        .lock()
        .map_err(|_| fail("store_state_poisoned", "任務紀錄狀態異常"))?;
    store
        .finish(&task_id, generation, &result)
        .map_err(|error| CliError::from(error).noting("；任務結果未確認，請勿直接重跑"))?;
    let record = store.status(&task_id)?;
    let recorded_state = record.recorded_state;
    announced?;
    if recorded_state == "cancelled" {
        println!(
            "{}",
            serde_json::json!({"task_id":task_id,"state":recorded_state,"artifact_created":record.artifact_created})
        );
        writeln!(
            std::io::stderr(),
            "已停止本機任務；已建立的成果會保留，請核對後再接續。"
        )
        .map_err(|_| {
            fail(
                "task_output_failed",
                "停止結果無法輸出，請核對已保存的任務狀態",
            )
        })?;
        return Ok(ExitCode::from(130));
    }
    let report = match result {
        Err(TaskError::Cancelled { artifact_created }) => {
            println!(
                "{}",
                serde_json::json!({"task_id": task_id, "state": recorded_state, "artifact_created": artifact_created})
            );
            writeln!(
                std::io::stderr(),
                "本機任務已結束，但工具結果未確認；請核對紀錄與成果，不要直接重跑。"
            )
            .map_err(|_| {
                fail(
                    "task_output_failed",
                    "停止結果無法輸出，請核對已保存的任務狀態",
                )
            })?;
            return Ok(ExitCode::from(130));
        }
        other => other?,
    };
    if recorded_state == "outcome_unknown" {
        return Err(fail(
            "outcome_unknown",
            "工具結果未確認，請核對紀錄與成果後再接續",
        ));
    }
    println!(
        "{}",
        serde_json::json!({"task_id": task_id, "artifact": report.artifact, "assistant_message": report.assistant_message})
    );
    Ok(if report.artifact == ArtifactState::Verified {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    })
}
