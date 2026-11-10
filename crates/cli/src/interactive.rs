mod resume;
use resume::resume;
mod approvals;
mod decide;
use approvals::approvals;
use decide::decide;
mod cancel;
mod cancel_saved;
mod run;
mod start;
use cancel::cancel;
pub use run::run;
use start::start;

use crate::open_store;
use agent_platform_agent::store::StoreAccess;
use std::collections::HashSet;
use std::io::{self, Write};
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

#[derive(Default)]
struct State {
    task_id: Option<String>,
    list_cursor: Option<String>,
    listed: Vec<String>,
    shown: Option<(String, i64)>,
    child: Option<Child>,
    generation: u64,
    ready: bool,
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
        data.ready = true;
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

fn query(state: &Arc<Mutex<State>>, command: &str) -> Result<(), String> {
    let id = task_id(state)?;
    let store = open_store(StoreAccess::Read).map_err(|_| "無法讀取任務紀錄")?;
    if command == "/result" {
        let snapshot = store
            .result_snapshot(&id)
            .map_err(|error| error.to_string())?;
        let result = agent_platform_agent::result::TaskResult::inspect(snapshot)
            .map_err(|error| error.to_string())?;
        output(if result.verified {
            "成果重新驗證通過"
        } else {
            "成果尚未證明完成，請核對"
        });
        output(&format!("成果檔：{}", result.output.display()));
        output("執行時來源：");
        output(
            result
                .before
                .as_deref()
                .unwrap_or("沒有保存的來源內容，無法還原差異"),
        );
        output("目前成果：");
        output(&result.after);
        return Ok(());
    }
    if command == "/status" {
        let record = store.status(&id).map_err(|_| "無法查詢狀態")?;
        let label = match record.recorded_state.as_str() {
            "running" => "執行中（最後保存狀態）",
            "waiting_approval" => "等待批准",
            "cancel_requested" => "已要求停止，尚待確認",
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

fn list(state: &Arc<Mutex<State>>, argument: &str) -> Result<(), String> {
    let cursor = match argument {
        "" => None,
        "next" => Some(
            state
                .lock()
                .map_err(|_| "狀態無法讀取")?
                .list_cursor
                .clone()
                .ok_or("沒有下一頁；輸入 /list 重新查詢")?,
        ),
        _ => return Err("用法：/list 或 /list next".into()),
    };
    let page = open_store(StoreAccess::Read)
        .map_err(|_| "無法讀取任務清單")?
        .list(cursor.as_deref())
        .map_err(|_| "無法查詢任務清單；請核對任務紀錄")?;
    {
        let mut data = state.lock().map_err(|_| "狀態無法讀取")?;
        data.list_cursor = page.next_cursor.clone();
        data.listed = page.tasks.iter().map(|task| task.task_id.clone()).collect();
    }
    if page.tasks.is_empty() {
        output("目前沒有任務");
        return Ok(());
    }
    for (index, task) in page.tasks.into_iter().enumerate() {
        let label = match task.recorded_state.as_str() {
            "running" => "執行中（最後保存狀態）",
            "waiting_approval" => "等待批准",
            "cancel_requested" => "已要求停止，尚待確認",
            "succeeded" => "成果已驗證",
            "needs_review" => "成果待人工核對",
            "failed" => "失敗",
            "cancelled" => "已取消",
            "outcome_unknown" => "結果未確認，請核對成果",
            _ => "狀態未知",
        };
        let workspace = task
            .workspace
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "未保存工作區".into());
        output(&format!(
            "{}. 任務：{}　狀態：{label}　工作區：{workspace}",
            index + 1,
            task.task_id
        ));
    }
    output(if page.next_cursor.is_some() {
        "還有下一頁，輸入 /list next"
    } else {
        "已列完；輸入 /list 重新查詢"
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn display_worker() {
        let Ok(mode) = std::env::var("AGENT_DISPLAY_TEST_MODE") else {
            return;
        };
        if mode == "render" {
            for line in [
                "plain output",
                r#"{"artifact":"verified","assistant_message":"verified answer"}"#,
                r#"{"artifact":"awaiting_review","assistant_message":"review answer"}"#,
                r#"{"state":"cancelled"}"#,
                r#"{"other":"value"}"#,
            ] {
                show_result(line);
            }
            return;
        }
        let state = Arc::new(Mutex::new(State {
            task_id: Some(std::env::var("AGENT_DISPLAY_TEST_TASK").unwrap()),
            ..State::default()
        }));
        let command = std::env::var("AGENT_DISPLAY_TEST_QUERY").unwrap();
        query(&state, &command).unwrap();
        notify_pending(&state, &mut HashSet::new()).unwrap();
    }

    #[test]
    fn result_display_distinguishes_verified_review_cancelled_and_raw_output() {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "interactive::tests::display_worker",
                "--nocapture",
            ])
            .env("AGENT_DISPLAY_TEST_MODE", "render")
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        for expected in [
            "plain output",
            "成果已驗證。\n模型回覆：verified answer",
            "成果待人工核對。\n模型回覆：review answer",
            "任務已取消，請核對成果檔案。",
            r#"{"other":"value"}"#,
        ] {
            assert!(text.contains(expected), "{text}");
        }
    }

    fn capture_query(folder: &std::path::Path, task: &str, command: &str) -> String {
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "interactive::tests::display_worker",
                "--nocapture",
            ])
            .env("AGENT_DISPLAY_TEST_MODE", "query")
            .env("AGENT_DISPLAY_TEST_TASK", task)
            .env("AGENT_DISPLAY_TEST_QUERY", command)
            .env("AGENT_STATE_DIR", folder)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    #[test]
    fn result_query_displays_actual_content_and_missing_history_without_success() {
        use agent_platform_agent::{
            store::{ApprovalMode, TaskSpec, TaskStore},
            task::FileTask,
        };
        let folder = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(folder.path(), std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let output = folder.path().join("output");
        std::fs::write(&output, "actual artifact").unwrap();
        let spec = TaskSpec {
            workspace: folder.path().to_path_buf(),
            endpoint: "http://127.0.0.1:9/v1".into(),
            model: "test".into(),
            approval_mode: ApprovalMode::Preapproved,
            task: FileTask {
                prompt: "work".into(),
                input: folder.path().join("input"),
                output,
                expected: None,
                allow_read: true,
                allow_write: true,
            },
        };
        let mut store = TaskStore::open(&folder.path().join("tasks.sqlite3")).unwrap();
        let (task, _) = store.start_spec(&spec).unwrap();
        drop(store);
        let text = capture_query(folder.path(), &task, "/result");
        assert!(text.contains("actual artifact"), "{text}");
        assert!(text.contains("沒有保存的來源內容，無法還原差異"), "{text}");
        assert!(text.contains("成果尚未證明完成"), "{text}");
        assert!(!text.contains("成果重新驗證通過"), "{text}");
    }

    #[test]
    fn status_and_event_display_read_the_current_persisted_task() {
        let folder = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(folder.path(), std::fs::Permissions::from_mode(0o700))
                .unwrap();
        }
        let database = folder.path().join("tasks.sqlite3");
        let mut store = agent_platform_agent::store::TaskStore::open(&database).unwrap();
        let (task, _) = store.start().unwrap();
        drop(store);
        let connection = rusqlite::Connection::open(database).unwrap();
        for (state, label) in [
            ("running", "執行中（最後保存狀態）"),
            ("waiting_approval", "等待批准"),
            ("succeeded", "成果已驗證"),
            ("needs_review", "成果待人工核對"),
            ("failed", "失敗"),
            ("cancelled", "已取消"),
            ("outcome_unknown", "結果未確認，請核對成果"),
        ] {
            connection
                .execute(
                    "UPDATE tasks SET recorded_state=?1 WHERE task_id=?2",
                    rusqlite::params![state, task],
                )
                .unwrap();
            let text = capture_query(folder.path(), &task, "/status");
            assert!(text.contains(&format!("任務狀態：{label}")), "{text}");
            assert!(!text.contains("started"), "{text}");
            let events = capture_query(folder.path(), &task, "/events");
            assert!(events.contains("started"), "{events}");
            assert!(!events.contains("任務狀態："), "{events}");
        }
    }

    #[test]
    fn stale_watchers_are_rejected_and_idle_watchers_stop() {
        let state = Arc::new(Mutex::new(State {
            generation: 4,
            ..State::default()
        }));
        assert!(same_generation(&state, 4));
        assert!(!same_generation(&state, 3));
        let (send, receive) = std::sync::mpsc::channel();
        thread::spawn(move || {
            watch_approvals(&state, 4);
            send.send(()).unwrap();
        });
        receive.recv_timeout(Duration::from_secs(1)).unwrap();
    }
}
