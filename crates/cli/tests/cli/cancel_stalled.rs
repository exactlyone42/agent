use crate::{
    case::Case, completion::completion, fake_server::FakeServer,
    tool_call_message::tool_call_message, write_json::write_json,
};
use agent_platform_agent::store::TaskStore;
use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn stopped_executor_is_terminated_and_confirmed_within_five_seconds() {
    let case = Case::limited(Duration::from_secs(25));
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
    let mut original = case.spawn(case.cli(&["run", task.to_str().unwrap()]));
    let line = original.stderr_line(Duration::from_secs(5)).unwrap();
    let task_id = line.strip_prefix("任務編號：").unwrap().trim().to_owned();
    let reader = TaskStore::read(&case.database()).unwrap();
    let until = Instant::now() + Duration::from_secs(3);
    let mut pending = false;
    while !pending && Instant::now() < until {
        pending = reader.status(&task_id).unwrap().recorded_state == "waiting_approval";
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(pending);
    original.signal(libc::SIGSTOP);
    let status_file = format!("/proc/{}/status", original.process_id());
    let until = Instant::now() + Duration::from_secs(1);
    let mut stopped = false;
    while !stopped && Instant::now() < until {
        stopped = std::fs::read_to_string(&status_file)
            .unwrap()
            .lines()
            .any(|line| line.starts_with("State:\tT"));
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(stopped);
    let began = Instant::now();
    let first = case.spawn(case.cli(&["cancel", &task_id]));
    let second = case.spawn(case.cli(&["cancel", &task_id]));
    let cancelled = first.finish();
    let repeated = second.finish();
    assert_eq!(cancelled.code(), Some(0), "{}", cancelled.stderr);
    assert_eq!(repeated.code(), Some(0), "{}", repeated.stderr);
    assert!(began.elapsed() < Duration::from_secs(5));
    assert!(original.wait_for(Duration::from_millis(250)));
    assert!(!original.finish().status.success());
    let acknowledgement: serde_json::Value = serde_json::from_str(&cancelled.stdout).unwrap();
    assert_eq!(acknowledgement["confirmed"], true);
    assert_eq!(acknowledgement["confirmation_scope"], "local_task");
    assert_eq!(reader.status(&task_id).unwrap().recorded_state, "cancelled");
    assert!(!case.root().join("output").exists());
    assert_eq!(server.requests().len(), 1);
    let events = reader.events(&task_id, 0).unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == "cancel_requested")
            .count(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == "cancelled")
            .count(),
        1
    );
    let last_seq = reader.status(&task_id).unwrap().last_seq;
    let observation = Instant::now();
    while observation.elapsed() < Duration::from_secs(10) {
        assert!(!case.root().join("output").exists());
        assert_eq!(reader.status(&task_id).unwrap().last_seq, last_seq);
        assert_eq!(server.requests().len(), 1);
        std::thread::sleep(Duration::from_millis(50));
    }
}
