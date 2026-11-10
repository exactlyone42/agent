use crate::{
    case::Case, completion::completion, fake_server::FakeServer,
    tool_call_message::tool_call_message, write_json::write_json,
};
use agent_platform_agent::store::TaskStore;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

#[test]
fn instructions_are_saved_in_order_and_a_resend_is_not_saved_twice() {
    let case = Case::limited(Duration::from_secs(30));
    let server = FakeServer::start(|_, _, stream| {
        let call = tool_call_message(
            "pending-write",
            "write_output",
            &json!({"content":"fixed\n"}),
        );
        write_json(stream, &completion(call, "tool_calls"));
    });
    let task = case.write_task(&json!({
        "endpoint":server.endpoint(),"model":"fixture-model","approval_mode":"manual",
        "task":{"prompt":"create output","input":"unused","output":"output","allow_write":true,"expected":"fixed\n"}
    }));
    let original = case.spawn(case.cli(&["run", task.to_str().unwrap()]));
    let line = original.stderr_line(Duration::from_secs(5)).unwrap();
    let task_id = line.strip_prefix("任務編號：").unwrap().trim().to_owned();
    let reader = TaskStore::read(&case.database()).unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while reader.status(&task_id).unwrap().recorded_state != "waiting_approval" {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    let mut receipts = Vec::new();
    for (id, text) in [("m-1", "第一句"), ("m-2", "第二句"), ("m-1", "第一句")] {
        let outcome = case.run(&["instruct", &task_id, id, text]);
        assert_eq!(outcome.code(), Some(0), "{}", outcome.stderr);
        receipts.push(serde_json::from_str::<Value>(&outcome.stdout).unwrap());
    }
    assert_eq!(
        receipts
            .iter()
            .map(|r| (
                r["seq"].as_i64().unwrap(),
                r["duplicate"].as_bool().unwrap()
            ))
            .collect::<Vec<_>>(),
        [(1, false), (2, false), (1, true)]
    );
    assert!(receipts.iter().all(|r| r["applied"] == false));
    let connection = rusqlite::Connection::open(case.database()).unwrap();
    let saved: Vec<(String, i64, String)> = connection
        .prepare(
            "SELECT client_message_id,seq,content FROM instructions WHERE task_id=?1 ORDER BY seq",
        )
        .unwrap()
        .query_map([&task_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        saved,
        [
            ("m-1".to_owned(), 1, "第一句".to_owned()),
            ("m-2".to_owned(), 2, "第二句".to_owned())
        ]
    );
}

#[test]
fn a_reused_message_id_with_other_text_or_an_unknown_task_is_refused_and_saves_nothing() {
    let case = Case::limited(Duration::from_secs(30));
    let server = FakeServer::start(|_, _, stream| {
        let call = tool_call_message(
            "pending-write",
            "write_output",
            &json!({"content":"fixed\n"}),
        );
        write_json(stream, &completion(call, "tool_calls"));
    });
    let task = case.write_task(&json!({
        "endpoint":server.endpoint(),"model":"fixture-model","approval_mode":"manual",
        "task":{"prompt":"create output","input":"unused","output":"output","allow_write":true,"expected":"fixed\n"}
    }));
    let original = case.spawn(case.cli(&["run", task.to_str().unwrap()]));
    let line = original.stderr_line(Duration::from_secs(5)).unwrap();
    let task_id = line.strip_prefix("任務編號：").unwrap().trim().to_owned();
    let reader = TaskStore::read(&case.database()).unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while reader.status(&task_id).unwrap().recorded_state != "waiting_approval" {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(
        case.run(&["instruct", &task_id, "m-1", "原文"]).code(),
        Some(0)
    );
    let changed = case.run(&["instruct", &task_id, "m-1", "改過"]);
    assert_ne!(changed.code(), Some(0));
    assert!(
        changed.stderr.contains("task_conflict"),
        "{}",
        changed.stderr
    );
    let unknown = case.run(&["instruct", "no-such-task", "m-9", "x"]);
    assert!(
        unknown.stderr.contains("task_not_found"),
        "{}",
        unknown.stderr
    );
    let connection = rusqlite::Connection::open(case.database()).unwrap();
    let saved: Vec<(String, String)> = connection
        .prepare("SELECT client_message_id,content FROM instructions")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(saved, [("m-1".to_owned(), "原文".to_owned())]);
}
