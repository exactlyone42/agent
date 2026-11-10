use crate::{cli_error::CliError, fail};
use std::io::Write;

pub fn announce_task(task_id: &str, manual: bool) -> Result<(), CliError> {
    let mut message = format!("任務編號：{task_id}\n");
    if manual {
        message.push_str("逐次確認模式：請以 approvals 查閱這個任務，再用 approve 或 deny 提交決定及任務版本。\n");
    }
    let mut output = std::io::stderr().lock();
    output
        .write_all(message.as_bytes())
        .and_then(|()| output.flush())
        .map_err(|_| {
            fail(
                "task_output_failed",
                "任務訊息無法輸出，已要求停止；請核對保存結果",
            )
        })
}
