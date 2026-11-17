use agent_platform_agent::{
    store::{ApprovalMode, Claim, StoreError, TaskSpec, TaskStore},
    task::{FILE_LIMIT, FileTask, TaskError},
};
use rusqlite::Connection;
use std::path::PathBuf;

fn spec(mode: ApprovalMode, allowed: bool) -> TaskSpec {
    let workspace = std::env::temp_dir();
    TaskSpec {
        task: FileTask {
            prompt: "work".into(),
            input: workspace.join("input"),
            output: workspace.join("output"),
            expected: Some("result".into()),
            allow_read: allowed,
            allow_write: allowed,
        },
        workspace,
        endpoint: "http://127.0.0.1:1/v1".into(),
        model: "fixture-model".into(),
        approval_mode: mode,
    }
}

fn path() -> PathBuf {
    std::env::temp_dir().join(format!("agent-approval-{}.sqlite3", uuid::Uuid::new_v4()))
}

#[test]
fn approval_is_bound_to_revision_call_arguments_and_one_use()
-> Result<(), Box<dyn std::error::Error>> {
    let path = path();
    let mut store = TaskStore::open(&path)?;
    let (id, _) = store.start_spec(&spec(ApprovalMode::Manual, true))?;
    let mut prepared =
        store.prepare_intent(&id, 0, "call-1", "write_output", r#"{"content":"result"}"#)?;
    assert_eq!(store.status(&id)?.recorded_state, "waiting_approval");
    assert_eq!(store.claim(&prepared)?, Claim::Pending);
    assert!(store.decide(&prepared.approval_id, 2, true).is_err());
    assert_eq!(store.claim(&prepared)?, Claim::Pending);
    store.decide(&prepared.approval_id, 1, true)?;
    assert!(store.decide(&prepared.approval_id, 1, true).is_err());
    let original = prepared.args_hash.clone();
    prepared.args_hash = "other arguments".into();
    assert!(store.claim(&prepared).is_err());
    prepared.args_hash = original;
    prepared.task_id = "other task".into();
    assert!(store.claim(&prepared).is_err());
    prepared.task_id = id.clone();
    prepared.call_id = "other call".into();
    assert!(store.claim(&prepared).is_err());
    prepared.call_id = "call-1".into();
    assert_eq!(store.claim(&prepared)?, Claim::Granted);
    assert_eq!(store.claim(&prepared)?, Claim::Denied);
    assert_eq!(store.approvals(&id)?[0].uses_remaining, 0);
    assert!(store.decide(&prepared.approval_id, 1, false).is_err());
    store.receipt(&id, 0, "call-1", true)?;
    assert!(store.receipt(&id, 0, "call-1", true).is_err());
    assert!(
        store
            .prepare_intent(&id, 0, "call-1", "write_output", r#"{"content":"changed"}"#)
            .is_err()
    );
    let events = store.events(&id, 0)?;
    assert_eq!(
        events
            .iter()
            .map(|e| e.event_type.as_str())
            .collect::<Vec<_>>(),
        vec![
            "started",
            "approval_requested",
            "approval_granted",
            "tool_dispatched",
            "tool_succeeded"
        ]
    );
    assert_eq!(
        events.iter().map(|e| e.seq).collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn denial_expiry_policy_and_task_finish_prevent_claims() -> Result<(), Box<dyn std::error::Error>> {
    for case in [
        "denied",
        "expired",
        "revoked",
        "revoked_status",
        "policy_denied",
        "cancelled",
        "spec_changed",
        "policy_changed",
        "generation_changed",
    ] {
        let path = path();
        let mut store = TaskStore::open(&path)?;
        let (id, _) = store.start_spec(&spec(ApprovalMode::Manual, case != "policy_denied"))?;
        let prepared =
            store.prepare_intent(&id, 0, "call-1", "write_output", r#"{"content":"result"}"#)?;
        let connection = Connection::open(&path)?;
        match case {
            "denied" => store.decide(&prepared.approval_id, 1, false)?,
            "expired" => {
                connection.execute("UPDATE approvals SET expires_at=0", [])?;
                assert!(store.decide(&prepared.approval_id, 1, true).is_err());
            }
            "revoked" => {
                store.decide(&prepared.approval_id, 1, true)?;
                store.decide(&prepared.approval_id, 1, false)?;
            }
            "revoked_status" => {
                store.decide(&prepared.approval_id, 1, true)?;
                connection.execute("UPDATE approvals SET status='revoked'", [])?;
            }
            "cancelled" => {
                let generation = store.status(&id)?.generation;
                store.finish(
                    &id,
                    generation,
                    &Err(TaskError::Cancelled {
                        artifact_created: false,
                    }),
                )?;
                assert!(store.decide(&prepared.approval_id, 1, true).is_err());
            }
            "spec_changed" => {
                store.decide(&prepared.approval_id, 1, true)?;
                connection.execute("UPDATE task_specs SET spec_revision=2", [])?;
            }
            "policy_changed" => {
                store.decide(&prepared.approval_id, 1, true)?;
                connection.execute("UPDATE task_specs SET policy_revision=2", [])?;
            }
            "generation_changed" => {
                store.decide(&prepared.approval_id, 1, true)?;
                connection.execute("UPDATE tasks SET generation=2", [])?;
            }
            _ => {
                assert!(store.decide(&prepared.approval_id, 1, true).is_err());
            }
        }
        assert!(
            !matches!(store.claim(&prepared), Ok(Claim::Granted | Claim::Pending)),
            "{case}"
        );
        if matches!(case, "denied" | "expired" | "revoked" | "revoked_status") {
            assert!(
                matches!(store.claim(&prepared), Ok(Claim::Denied)),
                "{case}"
            );
        }
        let dispatched: i64 = connection.query_row(
            "SELECT count(*) FROM tool_intents WHERE status='dispatched'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(dispatched, 0);
        drop(connection);
        drop(store);
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[test]
fn intent_and_approval_claim_roll_back_together() -> Result<(), Box<dyn std::error::Error>> {
    let path = path();
    let mut store = TaskStore::open(&path)?;
    let (id, _) = store.start_spec(&spec(ApprovalMode::Preapproved, true))?;
    let prepared = store.prepare_intent(&id, 0, "call-1", "read_input", "{}")?;
    let connection = Connection::open(&path)?;
    connection.execute_batch("CREATE TRIGGER reject_dispatch BEFORE INSERT ON events WHEN NEW.event_type='tool_dispatched' BEGIN SELECT RAISE(ABORT,'injected failure'); END;")?;
    assert!(matches!(
        store.claim(&prepared),
        Err(StoreError::Database(_))
    ));
    assert_eq!(store.approvals(&id)?[0].uses_remaining, 1);
    let state: String =
        connection.query_row("SELECT status FROM tool_intents", [], |r| r.get(0))?;
    assert_eq!(state, "prepared");
    assert_eq!(store.status(&id)?.last_seq, 2);
    connection.execute_batch("DROP TRIGGER reject_dispatch")?;
    assert_eq!(store.claim(&prepared)?, Claim::Granted);
    drop(connection);
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn migration_preserves_v1_tasks_events_and_read_only_access()
-> Result<(), Box<dyn std::error::Error>> {
    let path = path();
    let connection = Connection::open(&path)?;
    connection.execute_batch(include_str!("../../../sql/store_v1.sql"))?;
    connection.execute_batch("PRAGMA application_id=1095780180; PRAGMA user_version=1; INSERT INTO tasks(task_id,recorded_state,last_seq) VALUES ('old','running',1); INSERT INTO events(event_id,task_id,seq,event_type) VALUES ('old-event','old',1,'started');")?;
    drop(connection);
    let reader = TaskStore::read(&path)?;
    assert_eq!(reader.events("old", 0)?[0].event_id, "old-event");
    assert_eq!(reader.status("old")?.schema_version, 1);
    drop(reader);
    let store = TaskStore::open(&path)?;
    assert_eq!(store.events("old", 0)?[0].event_id, "old-event");
    assert_eq!(store.events("old", 0)?[0].schema_version, 1);
    assert_eq!(store.status("old")?.schema_version, 2);
    let connection = Connection::open(&path)?;
    let violations: i64 =
        connection.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
            r.get(0)
        })?;
    assert_eq!(violations, 0);
    drop(connection);
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn v2_migration_preserves_approvals_and_rolls_back_on_failure()
-> Result<(), Box<dyn std::error::Error>> {
    for fail in [false, true] {
        let path = path();
        let connection = Connection::open(&path)?;
        connection.pragma_update(None, "foreign_keys", false)?;
        connection.execute_batch(include_str!("../../../sql/store_v1.sql"))?;
        connection.execute_batch(include_str!("../../../sql/store_v2.sql"))?;
        connection.execute_batch("PRAGMA application_id=1095780180; PRAGMA user_version=2;
            INSERT INTO tasks(task_id,recorded_state,last_seq) VALUES ('legacy','running',1);
            INSERT INTO events(event_id,task_id,seq,event_type) VALUES ('legacy-event','legacy',1,'started');
            INSERT INTO task_specs(task_id,spec_json,spec_hash) VALUES ('legacy','{}','saved-spec-hash');
            INSERT INTO tool_intents(intent_id,task_id,generation,call_id,tool_name,tool_version,args_json,args_hash,scope_path,status)
            VALUES ('legacy-intent','legacy',1,'call-1','read_input',1,'{}','saved-args-hash','/test/input','prepared');
            INSERT INTO approvals(approval_id,intent_id,spec_revision,policy_revision,spec_hash,args_hash,tool_version,status,decision_source,expires_at,uses_remaining)
            VALUES ('legacy-approval','legacy-intent',1,1,'saved-spec-hash','saved-args-hash',1,'approved','task_policy',unixepoch()+120,1);")?;
        assert!(TaskStore::write(&path).is_err());
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        assert_eq!(version, 2);
        if fail {
            connection.execute_batch("CREATE TABLE tool_intents_v3 (value TEXT)")?;
        }
        let result = TaskStore::open(&path);
        assert_eq!(result.is_err(), fail);
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        assert_eq!(version, if fail { 2 } else { 3 });
        let saved: (String, String, i64) = connection.query_row(
            "SELECT approval_id,args_hash,uses_remaining FROM approvals",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        assert_eq!(
            saved,
            ("legacy-approval".into(), "saved-args-hash".into(), 1)
        );
        let event: String =
            connection.query_row("SELECT event_id FROM events", [], |r| r.get(0))?;
        assert_eq!(event, "legacy-event");
        if !fail {
            let count: i64 =
                connection.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| {
                    r.get(0)
                })?;
            assert_eq!(count, 0);
            let step: i64 =
                connection.query_row("SELECT step_id FROM tool_intents", [], |r| r.get(0))?;
            assert_eq!(step, 0);
        }
        drop(result);
        drop(connection);
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[test]
fn finalization_survives_approval_events_and_preserves_unknown_outcomes()
-> Result<(), Box<dyn std::error::Error>> {
    for dispatched in [false, true] {
        let path = path();
        let mut store = TaskStore::open(&path)?;
        let (id, generation) = store.start_spec(&spec(ApprovalMode::Manual, true))?;
        let prepared =
            store.prepare_intent(&id, 0, "call", "write_output", r#"{"content":"result"}"#)?;
        let mut approver = TaskStore::write(&path)?;
        approver.decide(&prepared.approval_id, 1, true)?;
        if dispatched {
            assert_eq!(store.claim(&prepared)?, Claim::Granted);
        }
        assert!(matches!(
            store.finish(&id, generation + 1, &Err(TaskError::MissingArtifact)),
            Err(StoreError::Conflict)
        ));
        store.finish(
            &id,
            generation,
            &Err(TaskError::Cancelled {
                artifact_created: dispatched,
            }),
        )?;
        assert_eq!(
            store.status(&id)?.recorded_state,
            if dispatched {
                "outcome_unknown"
            } else {
                "cancelled"
            }
        );
        let approval = &store.approvals(&id)?[0];
        assert_eq!(
            approval.intent_status,
            if dispatched {
                "outcome_unknown"
            } else {
                "cancelled"
            }
        );
        if !dispatched {
            assert_eq!(approval.status, "revoked");
        }
        assert!(approver.decide(&prepared.approval_id, 1, true).is_err());
        assert!(store.claim(&prepared).is_err());
        assert!(matches!(
            store.finish(&id, generation, &Err(TaskError::MissingArtifact)),
            Err(StoreError::Conflict)
        ));
        drop(approver);
        drop(store);
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[test]
fn corrupted_spec_is_fatal_but_malformed_tool_arguments_are_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let path = path();
    let mut store = TaskStore::open(&path)?;
    let (id, _) = store.start_spec(&spec(ApprovalMode::Manual, true))?;
    for (tool, args) in [
        ("read_input", "invalid"),
        ("write_output", "{}"),
        ("unknown", "{}"),
    ] {
        assert!(matches!(
            store.prepare_intent(&id, 0, "call", tool, args),
            Err(StoreError::InvalidToolCall)
        ));
    }
    let connection = Connection::open(&path)?;
    connection.execute(
        "UPDATE task_specs SET spec_hash='corrupted' WHERE task_id=?1",
        [&id],
    )?;
    assert!(matches!(
        store.prepare_intent(&id, 0, "call", "read_input", "{}"),
        Err(StoreError::InvalidSpec)
    ));
    assert!(store.approvals(&id)?.is_empty());
    assert_eq!(store.events(&id, 0)?.len(), 1);
    drop(connection);
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn argument_hash_is_sha256_hex_and_call_id_length_is_bounded()
-> Result<(), Box<dyn std::error::Error>> {
    let path = path();
    let mut store = TaskStore::open(&path)?;
    let (id, _) = store.start_spec(&spec(ApprovalMode::Preapproved, true))?;
    let prepared = store.prepare_intent(&id, 0, "call", "read_input", "{}")?;
    assert_eq!(
        prepared.args_hash,
        "44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    );
    store.prepare_intent(&id, 1, &"c".repeat(256), "read_input", "{}")?;
    for call_id in [String::new(), "c".repeat(257), "c".repeat(300)] {
        assert!(matches!(
            store.prepare_intent(&id, 2, &call_id, "read_input", "{}"),
            Err(StoreError::InvalidToolCall)
        ));
    }
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn escaped_content_at_the_file_limit_is_accepted() -> Result<(), Box<dyn std::error::Error>> {
    let path = path();
    let mut store = TaskStore::open(&path)?;
    let (id, _) = store.start_spec(&spec(ApprovalMode::Preapproved, true))?;
    let content = "\"".repeat(FILE_LIMIT as usize);
    let args = serde_json::json!({ "content": content }).to_string();
    store.prepare_intent(&id, 0, "call", "write_output", &args)?;
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn rejection_is_recorded_as_an_event() -> Result<(), Box<dyn std::error::Error>> {
    let path = path();
    let mut store = TaskStore::open(&path)?;
    let (id, _) = store.start_spec(&spec(ApprovalMode::Manual, true))?;
    let before = store.events(&id, 0)?.len();
    store.record_rejection(&id, "call-1")?;
    let events = store.events(&id, 0)?;
    assert_eq!(events.len(), before + 1);
    assert_eq!(
        events
            .last()
            .map(|e| (e.event_type.as_str(), e.related_id.as_deref())),
        Some(("tool_rejected", Some("call-1")))
    );
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn saved_spec_size_limit_is_exact() -> Result<(), Box<dyn std::error::Error>> {
    let path = path();
    let mut store = TaskStore::open(&path)?;
    let mut sized = spec(ApprovalMode::Manual, true);
    let base = serde_json::to_string(&sized)?.len();
    sized.task.prompt = "x".repeat(131072 - base + sized.task.prompt.len());
    assert_eq!(serde_json::to_string(&sized)?.len(), 131072);
    store.start_spec(&sized)?;
    sized.task.prompt.push('x');
    assert!(matches!(
        store.start_spec(&sized),
        Err(StoreError::InvalidSpec)
    ));
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}
