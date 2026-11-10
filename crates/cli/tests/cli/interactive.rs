use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[test]
fn slash_help_and_exit_work_in_interactive_session() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agent-platform"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"\n/help\n/run\n/unknown\n/status\n/events\n/approvals\n/exit\n/unknown\n")
        .unwrap();
    let result = child.wait_with_output().unwrap();
    let output = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success(), "{output}");
    assert!(output.contains("/help"), "{output}");
    assert!(output.contains("/exit"), "{output}");
    assert_eq!(output.matches("不認識的指令").count(), 2, "{output}");
    assert_eq!(output.matches("尚無目前任務").count(), 3, "{output}");
    assert!(!output.contains("任務仍在執行"), "{output}");
}

#[test]
fn slash_approval_requires_review_and_executes_after_approval() {
    approval_scenario("/approve");
    approval_scenario("/deny");
}

fn approval_scenario(decision: &str) {
    use crate::{
        case::Case, completion::completion, fake_server::FakeServer, text_message::text_message,
        tool_call_message::tool_call_message, write_json::write_json,
    };
    use serde_json::json;

    let case = Case::new();
    let artifact = case.root().join("output.txt");
    let server = FakeServer::start(|index, _, stream| {
        let reply = if index == 1 {
            completion(
                tool_call_message("write-1", "write_output", &json!({"content":"APPROVED"})),
                "tool_calls",
            )
        } else {
            completion(text_message("done"), "stop")
        };
        write_json(stream, &reply);
    });
    let task = case.write_task(&json!({"endpoint":server.endpoint(),"model":"fixture-model","approval_mode":"manual","task":{"prompt":"create output","input":"unused","output":artifact,"allow_write":true,"expected":"APPROVED"}}));
    let mut child = case
        .cli(&[])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    writeln!(input, "/approve extra").unwrap();
    writeln!(input, "/approve").unwrap();
    writeln!(input, "/deny").unwrap();
    let first = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(first.contains("互動模式"));
    let invalid = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(invalid.contains("不接受額外參數"));
    let denied = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(denied.contains("請先輸入 /approvals"));
    let denied = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(denied.contains("請先輸入 /approvals"));
    writeln!(input, "/run {}", task.display()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut reviewed = false;
    let mut shown_content = false;
    while Instant::now() < deadline {
        if let Ok(line) = receiver.recv_timeout(Duration::from_millis(250)) {
            shown_content |= line.contains("APPROVED");
            if line.starts_with("輸入 /approve 或 /deny") {
                reviewed = true;
                break;
            }
        }
    }
    assert!(reviewed, "approval not shown");
    assert!(shown_content, "approval contents not shown");
    assert!(!artifact.exists());
    let connection = rusqlite::Connection::open(case.database()).unwrap();
    let task_id: String = connection
        .query_row("SELECT task_id FROM tasks", [], |row| row.get(0))
        .unwrap();
    drop(connection);
    let events = case.run(&["events", &task_id]);
    assert!(events.status.success(), "{}", events.stderr);
    assert!(events.stdout.contains("started"), "{}", events.stdout);
    writeln!(input, "/status").unwrap();
    writeln!(input, "/events").unwrap();
    writeln!(input, "/approvals").unwrap();
    writeln!(input, "/exit").unwrap();
    let observed_until = Instant::now() + Duration::from_millis(750);
    let mut observed = String::new();
    while Instant::now() < observed_until {
        if let Ok(line) = receiver.recv_timeout(Duration::from_millis(20)) {
            observed.push_str(&line);
            observed.push('\n');
        }
    }
    assert!(observed.contains("任務狀態：等待批准"), "{observed}");
    assert!(observed.contains("started"), "{observed}");
    assert!(observed.contains("任務仍在執行"), "{observed}");
    assert_eq!(observed.matches("待確認動作：").count(), 1, "{observed}");
    writeln!(input, "{decision}").unwrap();
    while Instant::now() < deadline && server.requests().len() < 2 {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(server.requests().len(), 2);
    if decision == "/approve" {
        assert_eq!(std::fs::read_to_string(&artifact).unwrap(), "APPROVED");
    } else {
        assert!(!artifact.exists());
        assert!(server.requests()[1].to_string().contains("工具未獲批准"));
    }
    let expected = if decision == "/approve" {
        "成果已驗證"
    } else {
        "任務失敗："
    };
    let mut completed = String::new();
    while Instant::now() < deadline && !completed.contains(expected) {
        if let Ok(line) = receiver.recv_timeout(Duration::from_millis(20)) {
            completed.push_str(&line);
            completed.push('\n');
        }
    }
    assert!(completed.contains(expected), "{completed}");
    drop(input);
    let result = child.wait_with_output().unwrap();
    assert!(result.status.success());
}
