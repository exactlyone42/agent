use super::*;
use std::process::Command;

pub(super) fn cancel_saved(state: &Arc<Mutex<State>>, argument: &str) -> Result<(), String> {
    if running(state)? {
        return Err("本畫面已有執行中的任務，請用 /cancel 停止目前任務".into());
    }
    let index = argument
        .parse::<usize>()
        .ok()
        .and_then(|value| value.checked_sub(1))
        .ok_or("請使用 /list 顯示的正整數序號")?;
    let id = state
        .lock()
        .map_err(|_| "狀態無法讀取")?
        .listed
        .get(index)
        .cloned()
        .ok_or("清單沒有此序號，請輸入 /list")?;
    let executable = std::env::current_exe().map_err(|_| "找不到執行檔")?;
    let result = Command::new(executable)
        .args(["cancel", &id])
        .output()
        .map_err(|_| "無法啟動停止要求，請核對任務狀態")?;
    let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
    data.task_id = Some(id);
    data.shown = None;
    if result.status.success() {
        output("本機任務已取消；外部請求是否停止尚未確認，已建立的成果會保留，請核對後再接續。");
        return Ok(());
    }
    let message = String::from_utf8_lossy(&result.stderr)
        .lines()
        .last()
        .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .and_then(|value| value["message"].as_str().map(str::to_owned))
        .unwrap_or_else(|| "未能確認停止，請用 /status 核對任務狀態與成果".into());
    Err(message)
}
