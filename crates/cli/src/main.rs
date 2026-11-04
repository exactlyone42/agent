mod cli_error;
mod interactive;

use agent_harness_openai::OpenAi;
use agent_platform_agent::task::{
    ArtifactState, CancellationToken, FileTask, TaskError, run_file_task,
};
use agent_platform_agent::{
    approval::task_gate,
    single_link::single_link,
    store::{ApprovalMode, StoreAccess, TaskSpec, TaskStore},
    task_timeout::task_timeout,
};
use cli_error::CliError;
use serde::Deserialize;
use std::{
    io::Read,
    path::PathBuf,
    process::ExitCode,
    sync::{Arc, Mutex},
    time::Duration,
};

fn fail(code: &'static str, message: &str) -> CliError {
    CliError::new(code, message)
}

fn open_store(access: StoreAccess) -> Result<TaskStore, CliError> {
    let create = access == StoreAccess::Create;
    let directory = match std::env::var_os("AGENT_STATE_DIR") {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        Some(_) => return Err(fail("invalid_state_dir", "AGENT_STATE_DIR 不可為空")),
        None => directories::ProjectDirs::from("", "", "agent-platform")
            .map(|dirs| dirs.data_local_dir().to_path_buf())
            .ok_or_else(|| {
                fail(
                    "state_dir_unavailable",
                    "找不到使用者資料目錄，請設定 AGENT_STATE_DIR",
                )
            })?,
    };
    if !directory.is_absolute() {
        return Err(fail("invalid_state_dir", "AGENT_STATE_DIR 必須是絕對路徑"));
    }
    if create {
        let mut builder = std::fs::DirBuilder::new();
        builder.recursive(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&directory)
            .map_err(|_| fail("state_dir_unavailable", "無法建立任務資料目錄"))?;
    }
    let metadata = std::fs::symlink_metadata(&directory)
        .map_err(|_| fail("state_dir_unavailable", "找不到任務資料目錄"))?;
    if !metadata.is_dir() {
        return Err(fail(
            "invalid_state_dir",
            "任務資料目錄必須是一般目錄，不可使用符號連結",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(fail(
                "invalid_state_dir",
                "任務資料目錄不可讓其他使用者存取，請設定權限為 700",
            ));
        }
    }
    let path = directory.join("tasks.sqlite3");
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(fail(
                    "invalid_database_file",
                    "任務資料庫必須是一般檔案，不可使用符號連結",
                ));
            }
            if !single_link(&metadata) {
                return Err(fail("invalid_database_file", "任務資料庫不可使用硬連結"));
            }
        }
        Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(fail("database_unavailable", "找不到或無法讀取任務資料庫")),
    }
    match access {
        StoreAccess::Create => TaskStore::open(&path),
        StoreAccess::Read => TaskStore::read(&path),
        StoreAccess::Write => TaskStore::write(&path),
    }
    .map_err(CliError::from)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    endpoint: url::Url,
    model: String,
    task: FileTask,
    #[serde(default)]
    approval_mode: ApprovalMode,
}

async fn run() -> Result<ExitCode, CliError> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() {
        interactive::run().map_err(|message| fail("interactive_failed", &message))?;
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() == 1 && args[0] == "--help" {
        println!(
            "用法：agent-platform run <任務.json>\n      agent-platform status <任務編號>\n      agent-platform events <任務編號> [已讀序號]\n      agent-platform approvals <任務編號>\n      agent-platform approve <批准編號> <任務版本>\n      agent-platform deny <批准編號> <任務版本>\nevents 每次最多 100 筆，下一頁使用最後一筆的 seq 作已讀序號。\n任務 JSON 的 approval_mode 設 manual 可逐次確認工具動作；每次批准有效 120 秒。\n目前為開發入口，只連本機模型服務；讀寫須在任務中明確授權。尚非企業正式發布。"
        );
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() == 2 && args[0] == "status" {
        let task_id = args[1]
            .to_str()
            .ok_or_else(|| fail("invalid_argument", "任務編號格式錯誤"))?;
        let record = open_store(StoreAccess::Read)?.status(task_id)?;
        if record.recorded_state == "running" {
            eprintln!("這是最後保存的狀態，不能確認程序仍在執行；請先核對成果，不要直接重跑。");
        }
        println!(
            "{}",
            serde_json::to_string(&record)
                .map_err(|_| fail("output_failed", "無法輸出任務狀態"))?
        );
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() == 2 && args[0] == "approvals" {
        let task_id = args[1]
            .to_str()
            .ok_or_else(|| fail("invalid_argument", "任務編號格式錯誤"))?;
        let approvals = open_store(StoreAccess::Read)?.approvals(task_id)?;
        println!(
            "{}",
            serde_json::to_string(&approvals)
                .map_err(|_| fail("output_failed", "無法輸出批准紀錄"))?
        );
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() == 3 && (args[0] == "approve" || args[0] == "deny") {
        let approval_id = args[1]
            .to_str()
            .ok_or_else(|| fail("invalid_argument", "批准編號格式錯誤"))?;
        let revision = args[2]
            .to_str()
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0)
            .ok_or_else(|| fail("invalid_argument", "必須提供正整數的任務版本"))?;
        let approve = args[0] == "approve";
        open_store(StoreAccess::Write)?.decide(approval_id, revision, approve)?;
        println!(
            "{}",
            serde_json::json!({"approval_id":approval_id,"status":if approve { "approved" } else { "denied" }})
        );
        return Ok(ExitCode::SUCCESS);
    }
    if (args.len() == 2 || args.len() == 3) && args[0] == "events" {
        let task_id = args[1]
            .to_str()
            .ok_or_else(|| fail("invalid_argument", "任務編號格式錯誤"))?;
        let after_seq = match args.get(2) {
            Some(value) => value
                .to_str()
                .and_then(|value| value.parse::<i64>().ok())
                .ok_or_else(|| fail("invalid_argument", "事件位置格式錯誤"))?,
            None => 0,
        };
        for event in open_store(StoreAccess::Read)?.events(task_id, after_seq)? {
            println!(
                "{}",
                serde_json::to_string(&event)
                    .map_err(|_| fail("output_failed", "無法輸出任務事件"))?
            );
        }
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() != 2 || args[0] != "run" {
        return Err(fail("usage", "用法：agent-platform run <任務.json>"));
    }
    let mut bytes = Vec::new();
    agent_platform_agent::input::open_checked(std::path::Path::new(&args[1]), u64::MAX)
        .map_err(|_| fail("task_file_unreadable", "無法開啟任務檔"))?
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| fail("task_file_unreadable", "無法讀取任務檔"))?;
    if bytes.len() > 65536 {
        return Err(fail("task_file_too_large", "任務檔超過 64 KiB"));
    }
    let mut request: Request = serde_json::from_slice(&bytes)
        .map_err(|_| fail("invalid_task_file", "任務 JSON 格式錯誤或包含未知欄位"))?;
    let loopback = match request.endpoint.host() {
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        _ => false,
    };
    if !loopback
        || !matches!(request.endpoint.scheme(), "http" | "https")
        || !request.endpoint.username().is_empty()
        || request.endpoint.password().is_some()
        || request.endpoint.query().is_some()
        || request.endpoint.fragment().is_some()
        || request.model.trim().is_empty()
    {
        return Err(fail(
            "invalid_model_endpoint",
            "目前只接受明確的本機 IP 模型入口與非空模型名稱；網址不得夾帶憑證或查詢參數",
        ));
    }
    let key = match std::env::var("AGENT_API_KEY") {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => String::new(),
        Err(_) => return Err(fail("invalid_api_key", "AGENT_API_KEY 格式錯誤")),
    };
    let model = OpenAi::new(
        &request.endpoint,
        request.model.clone(),
        key,
        Duration::from_secs(60),
    )
    .map_err(|error| fail("model_client_failed", &error.0))?;
    let cancellation = CancellationToken::new();
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
    let workspace =
        std::env::current_dir().map_err(|_| fail("workspace_unavailable", "無法取得工作目錄"))?;
    request.task.input = workspace.join(&request.task.input);
    request.task.output = workspace.join(&request.task.output);
    let spec = TaskSpec {
        workspace,
        endpoint: request.endpoint.to_string(),
        model: request.model.clone(),
        task: request.task.clone(),
        approval_mode: request.approval_mode,
    };
    let mut store = open_store(StoreAccess::Create)?;
    let (task_id, generation) = store.start_spec(&spec)?;
    let store = Arc::new(Mutex::new(store));
    let gate = task_gate(
        store.clone(),
        task_id.clone(),
        cancellation.clone(),
        request.approval_mode == ApprovalMode::Manual,
    );
    eprintln!("任務編號：{task_id}");
    if request.approval_mode == ApprovalMode::Manual {
        eprintln!(
            "逐次確認模式：請以 approvals 查閱這個任務，再用 approve 或 deny 提交決定及任務版本。"
        );
    }
    let mut result = run_file_task(
        model,
        request.task,
        cancellation,
        gate.clone(),
        task_timeout(request.approval_mode),
    )
    .await;
    if matches!(result, Err(TaskError::Harness(_))) && gate.was_denied() {
        result = Err(TaskError::ApprovalDenied);
    }
    let mut store = store
        .lock()
        .map_err(|_| fail("store_state_poisoned", "任務紀錄狀態異常"))?;
    store
        .finish(&task_id, generation, &result)
        .map_err(|error| CliError::from(error).noting("；任務結果未確認，請勿直接重跑"))?;
    let recorded_state = store.status(&task_id)?.recorded_state;
    let report = match result {
        Err(TaskError::Cancelled { artifact_created }) => {
            println!(
                "{}",
                serde_json::json!({"task_id": task_id, "state": recorded_state, "artifact_created": artifact_created})
            );
            eprintln!("已停止本機任務；已建立的成果會保留，請核對後再接續。");
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

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{}", error.line());
            ExitCode::FAILURE
        }
    }
}
