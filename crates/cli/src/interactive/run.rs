use super::*;
use std::io::BufRead;

pub fn run() -> Result<(), String> {
    let state = Arc::new(Mutex::new(State::default()));
    output("Agent 互動模式。輸入 /help 查看指令。");
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|_| "無法讀取輸入")?;
        let input = line.trim();
        let (command, argument) = input.split_once(' ').unwrap_or((input, ""));
        if !matches!(command, "/run" | "/list" | "/resume" | "/cancel")
            && !argument.trim().is_empty()
        {
            output("此指令不接受額外參數，請輸入 /help");
            continue;
        }
        let result = match command {
            "/help" => {
                output(
                    "/version　查看版本與能力\n/list　查看任務清單；/list next 查看下一頁\n/run <任務.json>　啟動任務\n/resume [序號]　接續目前任務，或選擇清單中的任務\n/result　讀回成果與執行時來源\n/status　查看狀態\n/events　查看紀錄\n/approvals　查看待批准動作\n/approve　批准\n/deny　拒絕\n/cancel　停止目前任務；/cancel <序號> 停止清單中的任務\n/exit　離開",
                );
                Ok(())
            }
            "/version" => {
                output(&crate::version());
                Ok(())
            }
            "/run" if !argument.is_empty() => start(&state, "run", argument.trim()),
            "/resume" => resume(&state, argument.trim()),
            "/list" => list(&state, argument.trim()),
            "/approvals" => approvals(&state),
            "/approve" => decide(&state, true),
            "/deny" => decide(&state, false),
            "/cancel" if !argument.trim().is_empty() => {
                cancel_saved::cancel_saved(&state, argument.trim())
            }
            "/cancel" => cancel(&state),
            "/status" | "/events" | "/result" => query(&state, command),
            "/exit" if !running(&state)? => return Ok(()),
            "/exit" => Err("任務仍在執行，請先等待完成".into()),
            "" => Ok(()),
            _ => Err("不認識的指令，請輸入 /help".into()),
        };
        if let Err(error) = result {
            output(&error);
        }
    }
    if running(&state)? {
        output("輸入已關閉；正在停止任務，請核對已建立的成果。");
        let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
        if let Some(child) = data.child.as_mut() {
            drop(child.stdin.take());
            child.wait().map_err(|_| "無法等待任務結束")?;
        }
    }
    Ok(())
}
