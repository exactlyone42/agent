use agent_harness::{Message, Session};
use agent_platform_agent::store::{ApprovalMode, Claim, TaskSpec, TaskStore};
use agent_platform_agent::task::FileTask;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SPEC_LIMIT: usize = 131_072;
const SESSION_LIMIT: usize = 33_554_432;
const ARGS_LIMIT: usize = 6 * 1024 * 1024 + 1024;

fn sha(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn spec() -> TaskSpec {
    TaskSpec {
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
    }
}

fn database() -> PathBuf {
    std::env::temp_dir().join(format!("agent-limits-{}.sqlite3", uuid::Uuid::new_v4()))
}

fn padded(valid: &str, length: usize) -> String {
    format!("{valid}{}", " ".repeat(length - valid.len()))
}

fn succeed(store: &mut TaskStore, task: &str, step: i64) -> TestResult {
    let call = format!("call{step}");
    let intent =
        store.prepare_intent(task, 1, step, &call, "write_output", r#"{"content":"x"}"#)?;
    assert!(matches!(store.claim(&intent)?, Claim::Granted));
    store.receipt(task, 1, step, &call, true)?;
    Ok(())
}

#[test]
fn saved_call_arguments_are_accepted_at_the_limit_and_rejected_above() -> TestResult {
    let path = database();
    let mut store = TaskStore::open(&path)?;
    let (task, _) = store.start_spec(&spec())?;
    succeed(&mut store, &task, 0)?;
    let connection = rusqlite::Connection::open(&path)?;
    for (length, accepted) in [(ARGS_LIMIT, true), (ARGS_LIMIT + 1, false)] {
        let args = "a".repeat(length);
        connection.execute(
            "UPDATE tool_intents SET args_json=?1, args_hash=?2 WHERE task_id=?3",
            rusqlite::params![args, sha(&args), task],
        )?;
        assert_eq!(store.result_snapshot(&task).is_ok(), accepted, "{length}");
    }
    let args = "b".repeat(ARGS_LIMIT);
    connection.execute(
        "UPDATE tool_intents SET args_json=?1 WHERE task_id=?2",
        rusqlite::params![args, task],
    )?;
    assert!(store.result_snapshot(&task).is_err(), "hash mismatch");
    Ok(())
}

#[test]
fn saved_calls_are_accepted_at_64_and_rejected_at_65() -> TestResult {
    let path = database();
    let mut store = TaskStore::open(&path)?;
    let (task, _) = store.start_spec(&spec())?;
    succeed(&mut store, &task, 0)?;
    let connection = rusqlite::Connection::open(&path)?;
    let copy = |step: i64| {
        connection.execute(
            "INSERT INTO tool_intents (intent_id, task_id, schema_version, generation, call_id, tool_name, tool_version, args_json, args_hash, scope_path, status, step_id)
             SELECT 'copy' || ?2, task_id, schema_version, generation, 'copy' || ?2, tool_name, tool_version, args_json, args_hash, scope_path, status, ?2
             FROM tool_intents WHERE task_id=?1 AND step_id=0",
            rusqlite::params![task, step],
        )
    };
    for step in 1..64 {
        copy(step)?;
    }
    assert_eq!(store.result_snapshot(&task)?.successful_calls.len(), 64);
    copy(64)?;
    assert!(store.result_snapshot(&task).is_err());
    Ok(())
}

#[test]
fn saved_spec_is_accepted_at_the_limit_and_rejected_above_or_when_hash_differs() -> TestResult {
    let path = database();
    let mut store = TaskStore::open(&path)?;
    let (task, _) = store.start_spec(&spec())?;
    let valid = serde_json::to_string(&spec())?;
    let connection = rusqlite::Connection::open(&path)?;
    for (length, accepted) in [(SPEC_LIMIT, true), (SPEC_LIMIT + 1, false)] {
        let json = padded(&valid, length);
        connection.execute(
            "UPDATE task_specs SET spec_json=?1, spec_hash=?2 WHERE task_id=?3",
            rusqlite::params![json, sha(&json), task],
        )?;
        assert_eq!(store.result_snapshot(&task).is_ok(), accepted, "{length}");
    }
    connection.execute(
        "UPDATE task_specs SET spec_json=?1, spec_hash=?2 WHERE task_id=?3",
        rusqlite::params![valid, "0".repeat(64), task],
    )?;
    assert!(store.result_snapshot(&task).is_err(), "hash mismatch");
    Ok(())
}

#[test]
fn saved_session_is_accepted_at_the_limit_and_rejected_above() -> TestResult {
    let path = database();
    let mut store = TaskStore::open(&path)?;
    let (task, generation) = store.start_spec(&spec())?;
    let session = Session {
        revision: 1,
        messages: vec![Message::User("work".into())],
        next_turn: 0,
        completed: false,
        unresolved_effect: false,
    };
    let valid = serde_json::to_string(&session)?;
    let connection = rusqlite::Connection::open(&path)?;
    connection.execute(
        "INSERT INTO sessions (task_id, generation, revision, session_json) VALUES (?1, ?2, 1, ?3)",
        rusqlite::params![task, generation, valid],
    )?;
    for (length, accepted) in [(SESSION_LIMIT, true), (SESSION_LIMIT + 1, false)] {
        connection.execute(
            "UPDATE sessions SET session_json=?1 WHERE task_id=?2",
            rusqlite::params![padded(&valid, length), task],
        )?;
        assert_eq!(store.result_snapshot(&task).is_ok(), accepted, "{length}");
    }
    Ok(())
}

#[test]
fn schema_3_omits_history_and_schema_4_reads_it() -> TestResult {
    let path = database();
    let mut store = TaskStore::open(&path)?;
    let (task, generation) = store.start_spec(&spec())?;
    let session = Session {
        revision: 1,
        messages: vec![Message::User("work".into())],
        next_turn: 0,
        completed: false,
        unresolved_effect: false,
    };
    let connection = rusqlite::Connection::open(&path)?;
    connection.execute(
        "INSERT INTO sessions (task_id, generation, revision, session_json) VALUES (?1, ?2, 1, ?3)",
        rusqlite::params![task, generation, serde_json::to_string(&session)?],
    )?;
    drop(store);
    for (version, expected) in [(3, None), (4, Some(session))] {
        connection.pragma_update(None, "user_version", version)?;
        let reader = TaskStore::read(&path)?;
        let snapshot = reader.result_snapshot(&task)?;
        assert!(snapshot.spec.is_some());
        assert_eq!(snapshot.session, expected, "schema {version}");
        let stored_version: i64 =
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        assert_eq!(stored_version, version, "read must preserve schema version");
    }
    Ok(())
}
