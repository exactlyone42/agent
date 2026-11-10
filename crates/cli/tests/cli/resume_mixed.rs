use crate::{
    await_flag::await_flag, case::Case, completion::completion, fake_server::FakeServer,
    hold_until_disconnect::hold_until_disconnect, open_database::open_database,
    text_message::text_message, tool_call_message::tool_call_message, write_json::write_json,
};
use agent_platform_agent::store::TaskStore;
use serde_json::{Value, json};
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

#[test]
fn denied_then_written_task_resumes_with_results_paired_to_original_calls() {
    let case = Case::new();
    std::fs::write(case.root().join("input"), "source\n").unwrap();
    let ready = Arc::new(AtomicBool::new(false));
    let disconnected = Arc::new(AtomicBool::new(false));
    let server = FakeServer::start({
        let ready = ready.clone();
        let disconnected = disconnected.clone();
        move |index, _, stream| {
            if let Some(id) = ["denied-one", "denied-two", "written-one"].get(index - 1) {
                let call = tool_call_message(id, "write_output", &json!({"content":"fixed\n"}));
                write_json(stream, &completion(call, "tool_calls"));
                return;
            }
            if index == 4 {
                hold_until_disconnect(stream, &ready, &disconnected);
                return;
            }
            assert_eq!(index, 5);
            write_json(stream, &completion(text_message("done"), "stop"));
        }
    });
    let task = case.write_task(&json!({
        "endpoint":server.endpoint(),"model":"fixture-model","approval_mode":"manual",
        "task":{"prompt":"repair source","input":"input","output":"output","allow_write":true,"expected":"fixed\n"}
    }));
    let mut original = case.spawn(case.cli(&["run", task.to_str().unwrap()]));
    let line = original.stderr_line(Duration::from_secs(5)).unwrap();
    let task_id = line.strip_prefix("任務編號：").unwrap().trim().to_owned();
    assert!(
        original
            .stderr_line(Duration::from_secs(5))
            .unwrap()
            .contains("逐次確認模式")
    );
    let output = case.root().join("output");
    for action in ["deny", "deny", "approve"] {
        let until = Instant::now() + Duration::from_secs(5);
        let mut pending = None;
        while pending.is_none() && Instant::now() < until && !original.exited() {
            pending = TaskStore::read(&case.database())
                .unwrap()
                .approvals(&task_id)
                .unwrap()
                .into_iter()
                .find(|item| item.status == "pending");
            if pending.is_none() {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let pending = pending.expect("pending write approval");
        assert!(!output.exists());
        let decision = case.run(&[action, &pending.approval_id, "1"]);
        assert_eq!(decision.code(), Some(0), "{}", decision.stderr);
    }
    assert!(original.wait_flag(&ready, Duration::from_secs(5)));
    let modified = std::fs::metadata(&output).unwrap().modified().unwrap();
    let connection = open_database(&case.database());
    let before: String = connection
        .query_row("SELECT session_json FROM sessions", [], |row| row.get(0))
        .unwrap();
    let saved: Value = serde_json::from_str(&before).unwrap();
    let results: Vec<_> = saved["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|message| message.get("Tool"))
        .map(|reply| {
            (
                reply["call_id"].as_str().unwrap(),
                reply["is_error"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        results,
        [
            ("denied-one", true),
            ("denied-two", true),
            ("written-one", false)
        ]
    );
    original.stop_parent();
    assert!(!original.finish().status.success());
    let resumed = case.run(&["resume", &task_id]);
    assert_eq!(resumed.code(), Some(0), "{}", resumed.stderr);
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "fixed\n");
    assert_eq!(
        std::fs::metadata(&output).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(
        std::fs::read_to_string(case.root().join("input")).unwrap(),
        "source\n"
    );
    let archived: String = connection
        .query_row(
            "SELECT session_json FROM task_runs WHERE task_id=?1 AND generation=1",
            [&task_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(archived, before);
    let store = TaskStore::read(&case.database()).unwrap();
    assert_eq!(store.status(&task_id).unwrap().generation, 2);
    let approvals = store.approvals(&task_id).unwrap();
    assert_eq!(approvals.len(), 3);
    assert_eq!(
        approvals
            .iter()
            .filter(|item| item.intent_status == "succeeded")
            .count(),
        1
    );
    let requests = server.requests();
    assert_eq!(requests.len(), 5);
    let recovered: Vec<_> = requests[4]["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["role"] == "tool")
        .cloned()
        .collect();
    let expected: Vec<_> = saved["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|message| message.get("Tool"))
        .map(|reply| json!({"role":"tool","tool_call_id":reply["call_id"],"content":reply["content"]}))
        .collect();
    assert_eq!(recovered, expected);
    let progress = requests[4]["messages"].as_array().unwrap().last().unwrap()["content"]
        .as_str()
        .unwrap();
    assert!(
        progress.contains("write_output：成功 1 次，未執行 2 次"),
        "{progress}"
    );
    assert!(await_flag(&disconnected, Duration::from_secs(5)));
    let readback = case.run(&["result", &task_id]);
    assert_eq!(readback.code(), Some(0), "{}", readback.stderr);
    let result: Value = serde_json::from_str(&readback.stdout).unwrap();
    assert_eq!(result["verified"], true);
    assert_eq!(result["after"], "fixed\n");
}
