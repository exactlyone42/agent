use super::*;

pub(super) fn resume(state: &Arc<Mutex<State>>, argument: &str) -> Result<(), String> {
    let id = {
        let data = state.lock().map_err(|_| "狀態無法讀取")?;
        if argument.is_empty() {
            data.task_id
                .clone()
                .ok_or("請先輸入 /list，再用 /resume <序號> 選擇任務")?
        } else {
            let index = argument
                .parse::<usize>()
                .ok()
                .and_then(|value| value.checked_sub(1))
                .ok_or("請使用 /list 顯示的正整數序號")?;
            data.listed
                .get(index)
                .cloned()
                .ok_or("清單沒有此序號，請輸入 /list")?
        }
    };
    start(state, "resume", &id)
}
