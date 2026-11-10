use super::*;

pub(super) fn decide(state: &Arc<Mutex<State>>, approve: bool) -> Result<(), String> {
    let shown = {
        let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
        if data
            .child
            .as_ref()
            .is_some_and(|child| child.stdin.is_none())
        {
            return Err("已要求停止，不能再批准動作".into());
        }
        data.shown.take().ok_or("請先輸入 /approvals 查看動作")?
    };
    let mut store = open_store(StoreAccess::Write).map_err(|_| "無法讀取批准紀錄")?;
    store
        .decide(&shown.0, shown.1, approve)
        .map_err(|_| "批准已過期或狀態改變，請重新查看")?;
    output(if approve { "已批准" } else { "已拒絕" });
    Ok(())
}
