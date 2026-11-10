use crate::case::Case;
use agent_platform_agent::store::TaskStore;
use serde_json::json;
use std::{
    os::{fd::OwnedFd, unix::net::UnixStream},
    process::Stdio,
};

#[test]
fn closed_parent_progress_pipe_saves_cancelled_state_during_startup() {
    let case = Case::new();
    let task = case.write_task(&json!({"endpoint":"http://127.0.0.1:9/v1","model":"fixture","task":{"prompt":"work","input":"unused","output":"output.txt","allow_write":true}}));
    let (writer, reader) = UnixStream::pair().unwrap();
    drop(reader);
    let mut command = case.cli(&["run", task.to_str().unwrap()]);
    command
        .env("AGENT_CONTROL_PIPE", "1")
        .stdin(Stdio::null())
        .stderr(Stdio::from(OwnedFd::from(writer)));
    let output = command.output().unwrap();
    let page = TaskStore::read(&case.database())
        .unwrap()
        .list(None)
        .unwrap();
    assert_eq!(page.tasks.len(), 1);
    assert_eq!(page.tasks[0].recorded_state, "cancelled");
    assert_eq!(output.status.code(), Some(74));
    assert!(!case.root().join("output.txt").exists());
    let events = TaskStore::read(&case.database())
        .unwrap()
        .events(&page.tasks[0].task_id, 0)
        .unwrap();
    assert!(
        !events
            .iter()
            .any(|event| event.event_type == "tool_dispatched")
    );
}
