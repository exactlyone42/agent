use super::*;

pub(super) fn cancel(state: &Arc<Mutex<State>>) -> Result<(), String> {
    let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
    let child = data.child.as_mut().ok_or("尚無執行中的任務")?;
    if child.try_wait().map_err(|_| "無法查詢任務")?.is_some() {
        return Err("任務已結束，請查詢狀態與成果".into());
    }
    let input = child.stdin.take().ok_or("已要求停止，請等待保存結果")?;
    data.shown = None;
    drop(input);
    output("已要求停止；保存結果後請核對成果。");
    Ok(())
}
