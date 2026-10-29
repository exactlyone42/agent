use crate::case::Case;
use serde_json::Value;

pub fn stored_result(case: &Case, task_id: &str) -> (Value, Vec<Value>) {
    let status_run = case.run(&["status", task_id]);
    assert_eq!(status_run.code(), Some(0), "{}", status_run.stderr);
    let status: Value = serde_json::from_str(&status_run.stdout).expect("status json");
    assert_eq!(
        status_run.stderr.contains("最後保存的狀態"),
        status["recorded_state"] == "running"
    );
    let events_run = case.run(&["events", task_id, "0"]);
    assert_eq!(events_run.code(), Some(0), "{}", events_run.stderr);
    let events: Vec<Value> = events_run
        .stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("event json"))
        .collect();
    let last = status["last_seq"].as_i64().expect("last_seq");
    let sequence: Vec<i64> = events
        .iter()
        .map(|event| event["seq"].as_i64().expect("seq"))
        .collect();
    assert_eq!(sequence, (1..=last).collect::<Vec<_>>());
    assert_eq!(events[0]["event_type"], "started");
    assert!(events.iter().all(|event| event["task_id"] == task_id));
    (status, events)
}
