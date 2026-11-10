use super::*;

pub(super) fn approvals(state: &Arc<Mutex<State>>) -> Result<(), String> {
    if running(state)? && !state.lock().map_err(|_| "狀態無法讀取")?.ready {
        return Err("任務正在啟動，請等啟動完成後確認動作".into());
    }
    let stopping = state
        .lock()
        .map_err(|_| "狀態無法讀取")?
        .child
        .as_ref()
        .is_some_and(|child| child.stdin.is_none());
    if stopping {
        return Err("已要求停止，不能再批准動作".into());
    }
    let id = task_id(state)?;
    let store = open_store(StoreAccess::Read).map_err(|_| "無法讀取批准紀錄")?;
    let records = store.approvals(&id).map_err(|_| "無法查詢批准紀錄")?;
    let pending: Vec<_> = records
        .iter()
        .filter(|item| item.status == "pending")
        .collect();
    if pending.len() != 1 {
        state.lock().map_err(|_| "狀態無法讀取")?.shown = None;
        return Err(if pending.is_empty() {
            "目前沒有待批准動作"
        } else {
            "有多個待批准動作，請使用非互動指令逐筆處理"
        }
        .into());
    }
    let item = pending[0];
    let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
    output(&format!(
        "待確認動作：{}\n檔案：{}\n參數：{}\n輸入 /approve 或 /deny",
        item.tool_name, item.scope_path, item.args_json
    ));
    data.shown = Some((item.approval_id.clone(), item.spec_revision));
    Ok(())
}
