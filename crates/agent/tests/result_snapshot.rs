use agent_harness::{Message, Session, SessionStore};
use agent_platform_agent::{store::TaskStore, task_session::TaskSession};
use std::sync::{Arc, Mutex};

#[test]
fn result_snapshot_reads_history_without_writer_lock_or_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("agent-result-{}.sqlite3", uuid::Uuid::new_v4()));
    let mut store = TaskStore::open(&path)?;
    let (task, generation) = store.start()?;
    let shared = Arc::new(Mutex::new(store));
    let writer = TaskSession::new(shared, task.clone(), generation)?;
    let mut session = Session {
        revision: 0,
        messages: vec![Message::User("work".into())],
        next_turn: 0,
        completed: false,
        unresolved_effect: false,
    };
    session.revision = writer.save(&session)?;
    let before = std::fs::read(&path)?;
    let reader = TaskStore::read(&path)?;
    let snapshot = reader.result_snapshot(&task)?;
    assert_eq!(snapshot.record.task_id, task);
    assert!(snapshot.spec.is_none());
    assert_eq!(snapshot.session, Some(session));
    assert!(reader.result_snapshot("invalid").is_err());
    assert!(
        reader
            .result_snapshot(&uuid::Uuid::new_v4().to_string())
            .is_err()
    );
    assert_eq!(std::fs::read(&path)?, before);
    let connection = rusqlite::Connection::open(&path)?;
    connection.execute(
        "UPDATE sessions SET revision=revision+1 WHERE task_id=?1",
        [&task],
    )?;
    assert!(
        reader.result_snapshot(&task).is_err(),
        "inconsistent saved revision must be rejected"
    );
    Ok(())
}

#[test]
fn result_snapshot_requires_success_receipt_and_valid_arguments()
-> Result<(), Box<dyn std::error::Error>> {
    use agent_platform_agent::{
        store::{ApprovalMode, Claim, TaskSpec},
        task::FileTask,
    };
    let path = std::env::temp_dir().join(format!(
        "agent-result-call-{}.sqlite3",
        uuid::Uuid::new_v4()
    ));
    let mut store = TaskStore::open(&path)?;
    let spec = TaskSpec {
        workspace: std::env::temp_dir(),
        endpoint: "http://127.0.0.1/v1".into(),
        model: "test".into(),
        approval_mode: ApprovalMode::Preapproved,
        task: FileTask {
            prompt: "work".into(),
            input: std::env::temp_dir().join("input"),
            output: std::env::temp_dir().join("output"),
            expected: None,
            allow_read: true,
            allow_write: true,
        },
    };
    let (task, _) = store.start_spec(&spec)?;
    let args = r#"{"content":"real"}"#;
    let intent = store.prepare_intent(&task, 1, 0, "call", "write_output", args)?;
    assert!(store.result_snapshot(&task)?.successful_calls.is_empty());
    assert!(matches!(store.claim(&intent)?, Claim::Granted));
    assert!(store.result_snapshot(&task)?.successful_calls.is_empty());
    store.receipt(&task, 1, 0, "call", true)?;
    let snapshot = store.result_snapshot(&task)?;
    assert_eq!(snapshot.successful_calls.len(), 1);
    assert_eq!(snapshot.successful_calls[0].args_json, args);
    assert_eq!(snapshot.successful_calls[0].call_id, "call");
    let connection = rusqlite::Connection::open(&path)?;
    connection.execute(
        "UPDATE tool_intents SET args_json='{}' WHERE task_id=?1",
        [&task],
    )?;
    assert!(
        store.result_snapshot(&task).is_err(),
        "changed tool arguments must be rejected"
    );
    Ok(())
}

#[test]
fn result_inspection_reads_actual_output_and_rejects_changed_content()
-> Result<(), Box<dyn std::error::Error>> {
    use agent_platform_agent::{
        result::TaskResult,
        store::{ApprovalMode, TaskSpec},
        task::FileTask,
    };
    let root = std::env::temp_dir().join(format!("agent-inspect-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root)?;
    let output = root.join("output");
    std::fs::write(&output, "actual")?;
    let mut store = TaskStore::open(&root.join("tasks.sqlite3"))?;
    let spec = TaskSpec {
        workspace: root.clone(),
        endpoint: "http://127.0.0.1/v1".into(),
        model: "test".into(),
        approval_mode: ApprovalMode::Preapproved,
        task: FileTask {
            prompt: "work".into(),
            input: root.join("input"),
            output: output.clone(),
            expected: Some("actual".into()),
            allow_read: true,
            allow_write: true,
        },
    };
    let (task, _) = store.start_spec(&spec)?;
    let result = TaskResult::inspect(store.result_snapshot(&task)?)?;
    assert_eq!(result.after, "actual");
    assert_eq!(result.expected_matches, Some(true));
    assert!(
        !result.verified,
        "a matching file alone is not task success"
    );
    assert!(result.before.is_none());
    let mut missing = store.result_snapshot(&task)?;
    missing.record.recorded_state = "succeeded".into();
    assert!(
        !TaskResult::inspect(missing)?.verified,
        "saved success without history must not verify"
    );
    #[cfg(unix)]
    {
        let alias = root.join("alias");
        std::os::unix::fs::symlink(&output, &alias)?;
        let mut linked = store.result_snapshot(&task)?;
        linked.spec.as_mut().ok_or("missing spec")?.task.output = alias;
        assert!(
            TaskResult::inspect(linked).is_err(),
            "symlink artifact must be rejected"
        );
    }
    std::fs::write(&output, "wrong")?;
    assert!(
        TaskResult::inspect(store.result_snapshot(&task)?).is_err(),
        "changed artifact must fail independent verification"
    );
    Ok(())
}
