use crate::{
    case::Case, completion::completion, fake_server::FakeServer,
    tool_call_message::tool_call_message, write_json::write_json,
};
use agent_platform_agent::store::TaskStore;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

#[test]
fn corrupt_owner_identity_is_refused_and_a_different_task_is_not_terminated() {
    let case = Case::limited(Duration::from_secs(35));
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
    let mut target = case.spawn(case.cli(&["run", task.to_str().unwrap()]));
    let line = target.stderr_line(Duration::from_secs(5)).unwrap();
    let task_id = line.strip_prefix("任務編號：").unwrap().trim().to_owned();
    let mut innocent = case.spawn(case.cli(&["run", task.to_str().unwrap()]));
    let line = innocent.stderr_line(Duration::from_secs(5)).unwrap();
    let innocent_id = line.strip_prefix("任務編號：").unwrap().trim().to_owned();
    let reader = TaskStore::read(&case.database()).unwrap();
    let until = Instant::now() + Duration::from_secs(5);
    while reader.status(&task_id).unwrap().recorded_state != "waiting_approval"
        || reader.status(&innocent_id).unwrap().recorded_state != "waiting_approval"
    {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    target.signal(libc::SIGSTOP);
    let status_file = format!("/proc/{}/status", target.process_id());
    let until = Instant::now() + Duration::from_secs(1);
    while !std::fs::read_to_string(&status_file)
        .unwrap()
        .lines()
        .any(|line| line.starts_with("State:\tT"))
    {
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(5));
    }
    let owner_file = case.database().with_extension(format!("{task_id}.lock"));
    let original = std::fs::read(&owner_file).unwrap();
    let original_record: Value = serde_json::from_slice(&original).unwrap();
    let mut wrong_generation = original_record.clone();
    wrong_generation["generation"] = json!(original_record["generation"].as_i64().unwrap() + 1);
    let mut reused_pid = original_record.clone();
    reused_pid["process_start"] = json!(original_record["process_start"].as_u64().unwrap() + 1);
    let mut wrong_boot = original_record.clone();
    wrong_boot["boot_id"] = json!("different-boot");
    let innocent_file = case
        .database()
        .with_extension(format!("{innocent_id}.lock"));
    let mut foreign_owner: Value =
        serde_json::from_slice(&std::fs::read(innocent_file).unwrap()).unwrap();
    foreign_owner["task_id"] = json!(task_id);
    for record in [wrong_generation, reused_pid, wrong_boot, foreign_owner] {
        std::fs::write(&owner_file, serde_json::to_vec(&record).unwrap()).unwrap();
        let refused = case.run(&["cancel", &task_id]);
        assert_eq!(refused.code(), Some(1), "{}", refused.stderr);
        assert!(refused.stdout.is_empty());
        let error: Value = serde_json::from_str(refused.stderr.lines().last().unwrap()).unwrap();
        assert_eq!(error["error_code"], "cancel_unconfirmed");
        assert!(!target.exited());
        assert!(!innocent.exited());
        assert_eq!(
            reader.status(&innocent_id).unwrap().recorded_state,
            "waiting_approval"
        );
        assert!(!case.root().join("output").exists());
    }
    std::fs::write(owner_file, original).unwrap();
    let stopped = case.run(&["cancel", &task_id]);
    assert_eq!(stopped.code(), Some(0), "{}", stopped.stderr);
    assert!(!target.finish().status.success());
    assert!(!innocent.exited());
    let stopped = case.run(&["cancel", &innocent_id]);
    assert_eq!(stopped.code(), Some(0), "{}", stopped.stderr);
    assert_eq!(innocent.finish().code(), Some(130));
    assert!(!case.root().join("output").exists());
    assert_eq!(server.requests().len(), 2);
}
