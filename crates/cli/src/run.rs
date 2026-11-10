use super::*;

pub(super) async fn run() -> Result<ExitCode, CliError> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() {
        interactive::run().map_err(|message| fail("interactive_failed", &message))?;
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() == 1 && args[0] == "version" {
        println!("{}", version());
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() == 1 && args[0] == "--help" {
        println!(
            "用法：agent-platform version\n      agent-platform list [清單位置]\n      agent-platform run <任務.json>\n      agent-platform resume <任務編號>\n      agent-platform cancel <任務編號>\n      agent-platform instruct <任務編號> <訊息編號> <文字>\n      agent-platform result <任務編號>\n      agent-platform status <任務編號>\n      agent-platform events <任務編號> [已讀序號] [--follow]\n      agent-platform approvals <任務編號>\n      agent-platform approve <批准編號> <任務版本>\n      agent-platform deny <批准編號> <任務版本>\nevents 每次最多 100 筆，下一頁使用最後一筆的 seq 作已讀序號；--follow 持續讀取，Ctrl+C 只停止讀取。\n任務 JSON 的 approval_mode 設 manual 可逐次確認工具動作；每次批准有效 120 秒。\n目前為開發入口，只連本機模型服務；讀寫須在任務中明確授權。尚非企業正式發布。"
        );
        return Ok(ExitCode::SUCCESS);
    }
    if (args.len() == 1 || args.len() == 2) && args[0] == "list" {
        let cursor = args
            .get(1)
            .map(|value| {
                value
                    .to_str()
                    .ok_or_else(|| fail("invalid_argument", "任務清單位置格式錯誤"))
            })
            .transpose()?;
        let page = open_store(StoreAccess::Read)?.list(cursor)?;
        println!(
            "{}",
            serde_json::to_string(&page).map_err(|_| fail("output_failed", "無法輸出任務清單"))?
        );
        return Ok(ExitCode::SUCCESS);
    }
    if args.len() == 2 && args[0] == "result" {
        let id = args[1]
            .to_str()
            .ok_or_else(|| fail("invalid_argument", "任務編號格式錯誤"))?;
        let snapshot = open_store(StoreAccess::Read)?.result_snapshot(id)?;
        let result = agent_platform_agent::result::TaskResult::inspect(snapshot)
            .map_err(|error| fail("result_unverified", &error.to_string()))?;
        println!(
            "{}",
            serde_json::to_string(&result).map_err(|_| fail("output_failed", "無法輸出成果"))?
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
    if args.len() == 2 && args[0] == "resume" {
        let id = args[1]
            .to_str()
            .ok_or_else(|| fail("invalid_argument", "任務編號格式錯誤"))?;
        let spec = open_store(StoreAccess::Read)?
            .result_snapshot(id)?
            .spec
            .ok_or_else(|| fail("invalid_task_spec", "缺少原任務規格，不能接續"))?;
        return crate::execute::execute(spec, Some(id)).await;
    }
    if args.len() == 2 && args[0] == "cancel" {
        let id = args[1]
            .to_str()
            .ok_or_else(|| fail("invalid_argument", "任務編號格式錯誤"))?;
        return crate::cancel_task::cancel_task(id).await;
    }
    if args.len() == 4 && args[0] == "instruct" {
        let text = |index: usize, message: &str| {
            args[index]
                .to_str()
                .ok_or_else(|| fail("invalid_argument", message))
        };
        return crate::instruct_task::instruct_task(
            text(1, "任務編號格式錯誤")?,
            text(2, "訊息編號格式錯誤")?,
            text(3, "指令內容格式錯誤")?,
        );
    }
    if args[0] == "events" {
        event_stream::EventStream::from_args(&args)?.run().await?;
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
    let workspace =
        std::env::current_dir().map_err(|_| fail("workspace_unavailable", "無法取得工作目錄"))?;
    request.task.input = workspace.join(&request.task.input);
    request.task.output = workspace.join(&request.task.output);
    crate::execute::execute(
        TaskSpec {
            workspace,
            endpoint: request.endpoint.to_string(),
            model: request.model,
            task: request.task,
            approval_mode: request.approval_mode,
        },
        None,
    )
    .await
}
