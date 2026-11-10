use agent_harness::{Message, RunError, Session, SessionStore};
use agent_platform_agent::{
    store::{ApprovalMode, Claim, StoreError, TaskSpec, TaskStore},
    task::{ArtifactState, FileTask, TaskError, TaskReport},
    task_session::TaskSession,
};
use std::sync::{Arc, Mutex};

#[test]
fn cancellation_pins_generation_blocks_dispatch_and_waits_for_owner_and_receipts()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::temp_dir().join(format!("agent-cancel-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory)?;
    let path = directory.join("tasks.sqlite3");
    let mut store = TaskStore::open(&path)?;
    let spec = TaskSpec {
        workspace: directory.clone(),
        endpoint: "http://127.0.0.1:1/v1".into(),
        model: "fixture-model".into(),
        approval_mode: ApprovalMode::Manual,
        task: FileTask {
            prompt: "work".into(),
            input: directory.join("input"),
            output: directory.join("output"),
            expected: None,
            allow_read: false,
            allow_write: true,
        },
    };
    for phase in [
        "pending",
        "gate_race",
        "receipted",
        "unsettled",
        "completion_race",
    ] {
        let (task_id, generation) = store.start_spec(&spec)?;
        let prepared = store.prepare_intent(
            &task_id,
            generation,
            0,
            "write",
            "write_output",
            r#"{"content":"fixed"}"#,
        )?;
        if !matches!(phase, "pending" | "gate_race") {
            store.decide(&prepared.approval_id, 1, true)?;
            assert!(matches!(store.claim(&prepared)?, Claim::Granted));
        }
        let shared = Arc::new(Mutex::new(store));
        let owner = TaskSession::new(shared.clone(), task_id.clone(), generation)?;
        let mut database = shared.lock().unwrap();
        let before = database.status(&task_id)?.last_seq;
        assert!(matches!(
            database.request_cancel(&task_id, generation + 1),
            Err(StoreError::Conflict)
        ));
        assert_eq!(database.status(&task_id)?.last_seq, before);
        database.request_cancel(&task_id, generation)?;
        database.request_cancel(&task_id, generation)?;
        assert_eq!(database.status(&task_id)?.last_seq, before + 1);
        assert_eq!(
            database.status(&task_id)?.recorded_state,
            "cancel_requested"
        );
        assert!(matches!(
            database.prepare_intent(
                &task_id,
                generation,
                1,
                "late",
                "write_output",
                r#"{"content":"late"}"#
            ),
            Err(StoreError::Conflict)
        ));
        assert!(matches!(
            database.decide(&prepared.approval_id, 1, true),
            Err(StoreError::Conflict)
        ));
        assert!(matches!(
            database.claim(&prepared),
            Err(StoreError::Conflict)
        ));
        if matches!(phase, "receipted" | "completion_race") {
            database.receipt(&task_id, generation, 0, "write", true)?;
        }
        drop(database);
        assert!(TaskSession::stopped(shared.clone(), task_id.clone(), generation)?.is_none());
        owner.save(&Session {
            revision: 0,
            messages: vec![Message::User("work".into())],
            next_turn: 0,
            completed: false,
            unresolved_effect: false,
        })?;
        let result = if phase == "completion_race" {
            Ok(TaskReport {
                artifact: ArtifactState::Verified,
                assistant_message: "done".into(),
            })
        } else if phase == "gate_race" {
            Err(TaskError::Harness(RunError::Stopped(
                "取消已在派工前保存".into(),
            )))
        } else {
            Err(TaskError::Cancelled {
                artifact_created: phase != "pending",
            })
        };
        shared
            .lock()
            .unwrap()
            .finish(&task_id, generation, &result)?;
        assert!(TaskSession::stopped(shared.clone(), task_id.clone(), generation)?.is_none());
        drop(owner);
        let stopped = TaskSession::stopped(shared.clone(), task_id.clone(), generation)?.unwrap();
        assert_eq!(
            stopped.recorded_state,
            if phase == "unsettled" {
                "outcome_unknown"
            } else {
                "cancelled"
            }
        );
        assert!(matches!(
            shared
                .lock()
                .unwrap()
                .request_cancel(&task_id, generation + 1),
            Err(StoreError::Conflict)
        ));
        store = Arc::try_unwrap(shared).ok().unwrap().into_inner().unwrap();
    }
    let (task_id, generation) = store.start_spec(&spec)?;
    let connection = rusqlite::Connection::open(&path)?;
    connection.execute_batch("CREATE TRIGGER reject_cancel BEFORE INSERT ON events WHEN NEW.event_type='cancel_requested' BEGIN SELECT RAISE(ABORT,'injected cancellation failure'); END;")?;
    let before = store.status(&task_id)?.last_seq;
    assert!(matches!(
        store.request_cancel(&task_id, generation),
        Err(StoreError::Database(_))
    ));
    assert_eq!(store.status(&task_id)?.recorded_state, "running");
    assert_eq!(store.status(&task_id)?.last_seq, before);
    drop(connection);
    drop(store);
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
