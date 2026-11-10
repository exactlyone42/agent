use agent_harness::ToolCall;
use agent_harness_approval::ApprovalStore;
use agent_platform_agent::{
    approval::TaskApprovals,
    store::{ApprovalMode, Claim, TaskSpec, TaskStore},
    task::FileTask,
};
use rusqlite::Connection;
use serde_json::json;
use std::sync::{Arc, Mutex};

#[test]
fn old_callbacks_cannot_mutate_a_new_execution() -> Result<(), Box<dyn std::error::Error>> {
    let mut violations = Vec::new();
    for case in [
        "prepare",
        "success",
        "failure",
        "reject",
        "old_success",
        "old_failure",
    ] {
        let path =
            std::env::temp_dir().join(format!("agent-generation-{}.sqlite3", uuid::Uuid::new_v4()));
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
        let shared = Arc::new(Mutex::new(store));
        let old = TaskApprovals::new(shared.clone(), id.clone(), 1);
        let call = ToolCall {
            id: "reused-call".into(),
            name: "read_input".into(),
            args: json!({}),
            bad_args: None,
        };
        let old_receipt = case.starts_with("old_");
        if old_receipt {
            let prepared = old.prepare(0, &call).map_err(|_| "old prepare failed")?;
            let mut store = shared.lock().map_err(|_| "poisoned")?;
            store.decide(&prepared.approval_id, 1, true)?;
            assert_eq!(store.claim(&prepared)?, Claim::Granted);
        }
        let connection = Connection::open(&path)?;
        connection.execute("UPDATE tasks SET generation=2 WHERE task_id=?1", [&id])?;
        let current = TaskApprovals::new(shared.clone(), id.clone(), 2);
        let receipt = matches!(case, "success" | "failure");
        let prepared = if receipt {
            let prepared = current
                .prepare(0, &call)
                .map_err(|_| "current prepare failed")?;
            let mut store = shared.lock().map_err(|_| "poisoned")?;
            store.decide(&prepared.approval_id, 1, true)?;
            assert_eq!(store.claim(&prepared)?, Claim::Granted);
            Some(prepared)
        } else {
            None
        };
        let before = shared.lock().map_err(|_| "poisoned")?.status(&id)?.last_seq;
        let rejected = match case {
            "prepare" => old.prepare(0, &call).is_err(),
            "reject" => old.reject(&call).is_err(),
            _ => old.receipt(0, &call, case.ends_with("success")).is_err(),
        };
        let store = shared.lock().map_err(|_| "poisoned")?;
        if !rejected || store.status(&id)?.last_seq != before {
            violations.push(case);
        }
        if receipt || old_receipt {
            assert_eq!(store.approvals(&id)?.len(), 1);
            assert_eq!(store.approvals(&id)?[0].intent_status, "dispatched");
        } else if !store.approvals(&id)?.is_empty() {
            violations.push("stale approval created");
        }
        drop(store);
        if let Some(prepared) = prepared {
            assert!(current.claim(&prepared).is_ok());
            current
                .receipt(0, &call, case == "success")
                .map_err(|_| "current receipt failed")?;
            let store = shared.lock().map_err(|_| "poisoned")?;
            assert_eq!(
                store.approvals(&id)?[0].intent_status,
                if case == "success" {
                    "succeeded"
                } else {
                    "failed"
                }
            );
        } else if !old_receipt {
            let prepared = current
                .prepare(0, &call)
                .map_err(|_| "current prepare failed")?;
            let mut store = shared.lock().map_err(|_| "poisoned")?;
            assert_eq!(store.claim(&prepared)?, Claim::Pending);
            store.decide(&prepared.approval_id, 1, false)?;
            assert_eq!(store.claim(&prepared)?, Claim::Denied);
        }
        drop(old);
        drop(current);
        drop(shared);
        drop(connection);
        std::fs::remove_file(path)?;
    }
    assert!(
        violations.is_empty(),
        "stale callbacks changed the new execution: {violations:?}"
    );
    Ok(())
}
