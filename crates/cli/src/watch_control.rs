use crate::{cli_error::CliError, fail};
use agent_platform_agent::task::CancellationToken;
use std::io::Read;

pub(crate) const CONTROL_PIPE: &str = "AGENT_CONTROL_PIPE";

pub fn watch_control(cancellation: CancellationToken) -> Result<(), CliError> {
    let Some(value) = std::env::var_os(CONTROL_PIPE) else {
        return Ok(());
    };
    if value != "1" {
        return Err(fail("invalid_control_pipe", "任務停止管線設定無效"));
    }
    std::thread::Builder::new()
        .name("task-control".into())
        .spawn(move || {
            let guard = cancellation.drop_guard();
            let result = std::io::stdin().read(&mut [0u8; 1]);
            if result.is_err() {
                eprintln!(
                    "{}",
                    fail("control_pipe_failed", "任務停止管線無法讀取，已要求停止").line()
                );
            }
            drop(guard);
        })
        .map(|_| ())
        .map_err(|_| {
            fail(
                "control_pipe_unavailable",
                "無法啟動任務停止管線，任務未啟動",
            )
        })
}
