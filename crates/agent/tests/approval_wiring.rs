use agent_harness::ToolCall;
use agent_harness_approval::{ApprovalStore, Policy};
use agent_platform_agent::{
    approval::{TaskApprovals, task_messages},
    store::{APPROVAL_TTL_SECONDS, ApprovalMode, TOOL_CALL_LIMIT, TaskSpec, TaskStore},
    task::FileTask,
};
use serde_json::json;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

#[test]
fn gate_defaults_match_store_limits() {
    let policy = Policy::default();
    assert_eq!(policy.max_calls, TOOL_CALL_LIMIT as usize);
    assert_eq!(
        policy.approval_wait,
        Duration::from_secs(APPROVAL_TTL_SECONDS as u64)
    );
}

#[test]
fn task_messages_are_the_chinese_ones() {
    assert_eq!(task_messages().cancelled, "任務已要求停止");
}

#[test]
fn rejecting_a_call_records_it_in_the_task() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("agent-wiring-{}.sqlite3", uuid::Uuid::new_v4()));
    let mut store = TaskStore::open(&path)?;
    let workspace = std::env::temp_dir();
    let (id, _) = store.start_spec(&TaskSpec {
        task: FileTask {
            prompt: "work".into(),
            input: workspace.join("input"),
            output: workspace.join("output"),
            expected: None,
            allow_read: true,
            allow_write: true,
        },
        workspace,
        endpoint: "http://127.0.0.1:1/v1".into(),
        model: "fixture-model".into(),
        approval_mode: ApprovalMode::Manual,
    })?;
    let store = Arc::new(Mutex::new(store));
    let approvals = TaskApprovals::new(store.clone(), id.clone(), 1);
    let call = ToolCall {
        id: "call-1".into(),
        name: "write_output".into(),
        args: json!({}),
        bad_args: None,
    };
    approvals.reject(&call).map_err(|_| "reject failed")?;
    let events = store.lock().map_err(|_| "poisoned")?.events(&id, 0)?;
    assert!(events.iter().any(|e| e.event_type == "tool_rejected"));
    std::fs::remove_file(path)?;
    Ok(())
}
