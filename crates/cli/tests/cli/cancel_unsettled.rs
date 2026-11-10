use crate::{
    case::Case, compile_fixture::compile_fixture, completion::completion, fake_server::FakeServer,
    sync_pipes::SyncPipes, tool_call_message::tool_call_message, write_json::write_json,
};
use agent_platform_agent::store::TaskStore;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

#[test]
fn killed_during_file_sync_preserves_unknown_effect_and_never_resends_write() {
    let case = Case::limited(Duration::from_secs(30));
    let server = FakeServer::start(|_, _, stream| {
        let call = tool_call_message(
            "blocked-write",
            "write_output",
            &json!({"content":"fixed\n"}),
        );
        write_json(stream, &completion(call, "tool_calls"));
    });
    let task = case.write_task(&json!({
        "endpoint":server.endpoint(),"model":"fixture-model","approval_mode":"manual",
        "task":{"prompt":"create output","input":"unused","output":"output","allow_write":true,"expected":"fixed\n"}
    }));
    let output = case.root().join("output");
    let library = compile_fixture(case.root());
    let mut command = case.cli(&["run", task.to_str().unwrap()]);
    let mut sync = SyncPipes::attach(&mut command, &library, &output);
    let mut original = case.spawn(command);
    let line = original.stderr_line(Duration::from_secs(5)).unwrap();
    let task_id = line.strip_prefix("任務編號：").unwrap().trim().to_owned();
    let reader = TaskStore::read(&case.database()).unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    let mut pending = None;
    while pending.is_none() && Instant::now() < until {
        pending = reader
            .approvals(&task_id)
            .unwrap()
            .into_iter()
            .find(|item| item.status == "pending");
        std::thread::sleep(Duration::from_millis(10));
    }
    let pending = pending.unwrap();
    assert_eq!(
        case.run(&["approve", &pending.approval_id, "1"]).code(),
        Some(0)
    );
    assert!(sync.reached(&mut original));
    assert_eq!(
        reader.approvals(&task_id).unwrap()[0].intent_status,
        "dispatched"
    );
    assert_eq!(std::fs::read_to_string(&output).unwrap(), "fixed\n");
    let modified = std::fs::metadata(&output).unwrap().modified().unwrap();
    let began = Instant::now();
    let cancelled = case.run(&["cancel", &task_id]);
    assert_eq!(cancelled.code(), Some(1), "{}", cancelled.stderr);
    assert!(began.elapsed() < Duration::from_secs(5));
    assert!(cancelled.stdout.is_empty());
    let error: Value = serde_json::from_str(cancelled.stderr.lines().last().unwrap()).unwrap();
    assert_eq!(error["error_code"], "cancel_unconfirmed");
    assert!(original.wait_for(Duration::from_millis(250)));
    assert!(!original.finish().status.success());
    assert_eq!(
        reader.status(&task_id).unwrap().recorded_state,
        "outcome_unknown"
    );
    assert_eq!(
        reader.approvals(&task_id).unwrap()[0].intent_status,
        "outcome_unknown"
    );
    let resumed = case.run(&["resume", &task_id]);
    assert_eq!(resumed.code(), Some(1));
    assert!(resumed.stderr.contains("resume_unsettled"));
    let last_seq = reader.status(&task_id).unwrap().last_seq;
    let observation = Instant::now();
    while observation.elapsed() < Duration::from_secs(10) {
        assert_eq!(std::fs::read_to_string(&output).unwrap(), "fixed\n");
        assert_eq!(
            std::fs::metadata(&output).unwrap().modified().unwrap(),
            modified
        );
        assert_eq!(reader.status(&task_id).unwrap().last_seq, last_seq);
        assert_eq!(server.requests().len(), 1);
        std::thread::sleep(Duration::from_millis(50));
    }
}
