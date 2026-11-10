use crate::{cli_error::CliError, fail, open_store};
use agent_platform_agent::{store::StoreAccess, task_session::TaskSession};
use std::{
    process::ExitCode,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub async fn cancel_task(task_id: &str) -> Result<ExitCode, CliError> {
    let mut store = open_store(StoreAccess::Write)?;
    let generation = store.status(task_id)?.generation;
    store.request_cancel(task_id, generation)?;
    let store = Arc::new(Mutex::new(store));
    let deadline = Instant::now() + Duration::from_secs(5);
    #[cfg(target_os = "linux")]
    let mut force = crate::linux_cancellation::LinuxCancellation::new();
    let record = loop {
        if let Some(record) = TaskSession::stopped(store.clone(), task_id.to_owned(), generation)? {
            break record;
        }
        #[cfg(target_os = "linux")]
        force.advance(store.clone(), task_id, generation)?;
        if Instant::now() >= deadline {
            return Err(fail(
                "cancel_unconfirmed",
                "已要求停止，但尚未確認執行者釋放任務，請核對狀態與成果",
            ));
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    };
    if record.recorded_state != "cancelled" {
        return Err(fail(
            "cancel_unconfirmed",
            "任務已結束，但工具結果未確認，請核對紀錄與成果",
        ));
    }
    println!(
        "{}",
        serde_json::json!({"schema_version":1,"task_id":task_id,"generation":generation,"state":"cancelled","confirmed":true,"confirmation_scope":"local_task","external_stop_confirmed":false})
    );
    Ok(ExitCode::SUCCESS)
}
