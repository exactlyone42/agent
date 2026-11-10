use super::*;
use std::io::BufRead;
use std::process::{Command, Stdio};

pub(super) fn start(
    state: &Arc<Mutex<State>>,
    command: &str,
    argument: &str,
) -> Result<(), String> {
    let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
    if let Some(child) = data.child.as_mut()
        && child.try_wait().map_err(|_| "無法查詢任務")?.is_none()
    {
        return Err("已有任務執行中".into());
    }
    let executable = std::env::current_exe().map_err(|_| "找不到執行檔")?;
    let mut child = Command::new(executable)
        .args([command, argument])
        .env(crate::watch_control::CONTROL_PIPE, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "無法啟動任務")?;
    let stderr = child.stderr.take().ok_or("無法讀取任務訊息")?;
    let stdout = child.stdout.take().ok_or("無法讀取任務結果")?;
    data.task_id = (command == "resume").then(|| argument.to_owned());
    data.shown = None;
    data.ready = false;
    data.child = Some(child);
    data.generation = data.generation.wrapping_add(1);
    let generation = data.generation;
    let shared = Arc::clone(state);
    thread::spawn(move || {
        for line in io::BufReader::new(stderr).lines().map_while(Result::ok) {
            record_task(&shared, generation, &line);
        }
    });
    thread::spawn(move || {
        for line in io::BufReader::new(stdout).lines().map_while(Result::ok) {
            show_result(&line);
        }
    });
    let shared = Arc::clone(state);
    thread::spawn(move || watch_approvals(&shared, generation));
    Ok(())
}
