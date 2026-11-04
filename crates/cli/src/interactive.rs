use crate::open_store;
use agent_platform_agent::store::StoreAccess;
use std::collections::HashSet;
use std::io::{self, BufRead, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Default)]
struct State {
    task_id: Option<String>,
    shown: Option<(String, i64)>,
    child: Option<Child>,
    generation: u64,
}

fn output(line: &str) {
    println!("{line}");
    let _ = io::stdout().flush();
}

fn record_task(state: &Arc<Mutex<State>>, generation: u64, line: &str) {
    if let Some(id) = line.strip_prefix("任務編號：")
        && let Ok(mut data) = state.lock()
        && data.generation == generation
    {
        data.task_id = Some(id.trim().to_owned());
        output("任務已啟動。");
        return;
    }
    if line.starts_with("逐次確認模式：") {
        output("逐次確認模式：動作出現後請輸入 /approve 或 /deny。");
        return;
    }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(line)
        && let Some(message) = value.get("message").and_then(|value| value.as_str())
    {
        output(&format!("任務失敗：{message}"));
        return;
    }
    output(line);
}

fn show_result(line: &str) {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        output(line);
        return;
    };
    if let Some(message) = value
        .get("assistant_message")
        .and_then(|value| value.as_str())
    {
        let result = if value.get("artifact").and_then(|value| value.as_str()) == Some("verified") {
            "成果已驗證"
        } else {
            "成果待人工核對"
        };
        output(&format!("{result}。\n模型回覆：{message}"));
        return;
    }
    if value.get("state").and_then(|value| value.as_str()) == Some("cancelled") {
        output("任務已取消，請核對成果檔案。");
        return;
    }
    output(line);
}

fn running(state: &Arc<Mutex<State>>) -> Result<bool, String> {
    let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
    let Some(child) = data.child.as_mut() else {
        return Ok(false);
    };
    Ok(child.try_wait().map_err(|_| "無法查詢任務")?.is_none())
}

fn start(state: &Arc<Mutex<State>>, path: &str) -> Result<(), String> {
    let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
    if let Some(child) = data.child.as_mut()
        && child.try_wait().map_err(|_| "無法查詢任務")?.is_none()
    {
        return Err("已有任務執行中".into());
    }
    let executable = std::env::current_exe().map_err(|_| "找不到執行檔")?;
    let mut child = Command::new(executable)
        .args(["run", path])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "無法啟動任務")?;
    let stderr = child.stderr.take().ok_or("無法讀取任務訊息")?;
    let stdout = child.stdout.take().ok_or("無法讀取任務結果")?;
    data.task_id = None;
    data.shown = None;
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

fn watch_approvals(state: &Arc<Mutex<State>>, generation: u64) {
    let mut notified = HashSet::new();
    loop {
        if !running(state).unwrap_or(false) || !same_generation(state, generation) {
            return;
        }
        let _ = notify_pending(state, &mut notified);
        thread::sleep(Duration::from_millis(250));
    }
}

fn notify_pending(state: &Arc<Mutex<State>>, notified: &mut HashSet<String>) -> Result<(), String> {
    let id = task_id(state)?;
    let store = open_store(StoreAccess::Read).map_err(|_| "無法讀取批准紀錄")?;
    let records = store.approvals(&id).map_err(|_| "無法查詢批准紀錄")?;
    let pending: Vec<_> = records
        .iter()
        .filter(|item| item.status == "pending")
        .collect();
    if pending.len() != 1 || !notified.insert(pending[0].approval_id.clone()) {
        return Ok(());
    }
    approvals(state)
}

fn same_generation(state: &Arc<Mutex<State>>, generation: u64) -> bool {
    state.lock().is_ok_and(|data| data.generation == generation)
}

fn task_id(state: &Arc<Mutex<State>>) -> Result<String, String> {
    state
        .lock()
        .map_err(|_| "狀態無法讀取")?
        .task_id
        .clone()
        .ok_or("尚無目前任務".into())
}

fn approvals(state: &Arc<Mutex<State>>) -> Result<(), String> {
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
    output(&format!(
        "待確認動作：{}\n檔案：{}\n參數：{}\n輸入 /approve 或 /deny",
        item.tool_name, item.scope_path, item.args_json
    ));
    state.lock().map_err(|_| "狀態無法讀取")?.shown =
        Some((item.approval_id.clone(), item.spec_revision));
    Ok(())
}

fn decide(state: &Arc<Mutex<State>>, approve: bool) -> Result<(), String> {
    let shown = state
        .lock()
        .map_err(|_| "狀態無法讀取")?
        .shown
        .take()
        .ok_or("請先輸入 /approvals 查看動作")?;
    let mut store = open_store(StoreAccess::Write).map_err(|_| "無法讀取批准紀錄")?;
    store
        .decide(&shown.0, shown.1, approve)
        .map_err(|_| "批准已過期或狀態改變，請重新查看")?;
    output(if approve { "已批准" } else { "已拒絕" });
    Ok(())
}

fn query(state: &Arc<Mutex<State>>, command: &str) -> Result<(), String> {
    let id = task_id(state)?;
    let store = open_store(StoreAccess::Read).map_err(|_| "無法讀取任務紀錄")?;
    if command == "/status" {
        let record = store.status(&id).map_err(|_| "無法查詢狀態")?;
        let label = match record.recorded_state.as_str() {
            "running" => "執行中（最後保存狀態）",
            "waiting_approval" => "等待批准",
            "succeeded" => "成果已驗證",
            "needs_review" => "成果待人工核對",
            "failed" => "失敗",
            "cancelled" => "已取消",
            "outcome_unknown" => "結果未確認，請核對成果",
            _ => "狀態未知",
        };
        output(&format!("任務狀態：{label}"));
        return Ok(());
    }
    let events = store.events(&id, 0).map_err(|_| "無法查詢紀錄")?;
    for event in events {
        output(&format!(
            "{} {} {}",
            event.seq, event.created_at, event.event_type
        ));
    }
    Ok(())
}

pub fn run() -> Result<(), String> {
    let state = Arc::new(Mutex::new(State::default()));
    output("Agent 互動模式。輸入 /help 查看指令。");
    for line in io::stdin().lock().lines() {
        let line = line.map_err(|_| "無法讀取輸入")?;
        let input = line.trim();
        let (command, argument) = input.split_once(' ').unwrap_or((input, ""));
        if command != "/run" && !argument.trim().is_empty() {
            output("此指令不接受額外參數，請輸入 /help");
            continue;
        }
        let result = match command {
            "/help" => {
                output(
                    "/run <任務.json>　啟動任務\n/status　查看狀態\n/events　查看紀錄\n/approvals　查看待批准動作\n/approve　批准\n/deny　拒絕\n/exit　離開",
                );
                Ok(())
            }
            "/run" if !argument.is_empty() => start(&state, argument.trim()),
            "/approvals" => approvals(&state),
            "/approve" => decide(&state, true),
            "/deny" => decide(&state, false),
            "/status" | "/events" => query(&state, command),
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
        output("輸入已關閉；等待執行中的任務結束。");
        let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
        if let Some(child) = data.child.as_mut() {
            child.wait().map_err(|_| "無法等待任務結束")?;
        }
    }
    Ok(())
}
