use agent_platform_agent::{
    store::{StoreError, TaskStore},
    task::{ArtifactState, TaskError, TaskReport},
};
use rusqlite::Connection;
use std::{path::PathBuf, time::Instant};

fn database_path() -> PathBuf {
    std::env::temp_dir().join(format!("agent-store-{}.sqlite3", uuid::Uuid::new_v4()))
}

fn verified() -> Result<TaskReport, TaskError> {
    Ok(TaskReport {
        artifact: ArtifactState::Verified,
        assistant_message: "must never be saved in events".into(),
    })
}

#[test]
fn results_and_ordered_events_survive_reopen() -> Result<(), Box<dyn std::error::Error>> {
    let path = database_path();
    for (result, state) in [
        (verified(), "succeeded"),
        (
            Ok(TaskReport {
                artifact: ArtifactState::AwaitingReview,
                assistant_message: String::new(),
            }),
            "needs_review",
        ),
        (Err(TaskError::MissingArtifact), "failed"),
        (Err(TaskError::IncompleteArtifact), "failed"),
        (
            Err(TaskError::Cancelled {
                artifact_created: true,
            }),
            "cancelled",
        ),
        (Err(TaskError::Timeout), "outcome_unknown"),
        (Err(TaskError::State), "outcome_unknown"),
    ] {
        let mut store = TaskStore::open(&path)?;
        let task_id = store.start()?.0;
        uuid::Uuid::parse_str(&task_id)?;
        assert_eq!(store.status(&task_id)?.recorded_state, "running");
        store.finish(&task_id, 1, &result)?;
        drop(store);
        let mut reopened = TaskStore::open(&path)?;
        let record = reopened.status(&task_id)?;
        assert_eq!(
            record.artifact_created,
            match &result {
                Ok(_)
                | Err(
                    TaskError::IncompleteArtifact
                    | TaskError::Cancelled {
                        artifact_created: true,
                    },
                ) => Some(true),
                Err(TaskError::MissingArtifact) => Some(false),
                _ => None,
            }
        );
        assert_eq!(record.recorded_state, state);
        assert_eq!(record.last_seq, 2);
        assert_eq!(record.schema_version, 2);
        assert_eq!(record.generation, 1);
        assert!(record.updated_at.ends_with('Z'));
        let events = reopened.events(&task_id, 0)?;
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].seq, 1);
        assert_eq!(events[0].event_type, "started");
        assert_eq!(events[1].seq, 2);
        assert_eq!(events[1].event_type, state);
        assert_ne!(events[0].event_id, events[1].event_id);
        assert_eq!(reopened.events(&task_id, 1)?.len(), 1);
        assert!(reopened.events(&task_id, 2)?.is_empty());
        assert!(matches!(
            reopened.finish(&task_id, 1, &verified()),
            Err(StoreError::Conflict)
        ));
        assert_eq!(reopened.status(&task_id)?.recorded_state, state);
        assert!(matches!(
            reopened.events(&task_id, -1),
            Err(StoreError::InvalidCursor)
        ));
        assert!(matches!(
            reopened.status("' OR 1=1 --"),
            Err(StoreError::NotFound)
        ));
        assert!(matches!(
            reopened.events("missing", 0),
            Err(StoreError::NotFound)
        ));
    }
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn event_failure_rolls_back_state_and_start() -> Result<(), Box<dyn std::error::Error>> {
    let path = database_path();
    let mut store = TaskStore::open(&path)?;
    let task_id = store.start()?.0;
    let connection = Connection::open(&path)?;
    connection.execute_batch("CREATE TRIGGER reject_event BEFORE INSERT ON events BEGIN SELECT RAISE(ABORT, 'injected failure'); END;")?;
    assert!(matches!(
        store.finish(&task_id, 1, &verified()),
        Err(StoreError::Database(_))
    ));
    assert_eq!(store.status(&task_id)?.recorded_state, "running");
    assert_eq!(store.status(&task_id)?.last_seq, 1);
    assert_eq!(store.events(&task_id, 0)?.len(), 1);
    assert!(store.start().is_err());
    let count: i64 = connection.query_row("SELECT count(*) FROM tasks", [], |r| r.get(0))?;
    assert_eq!(count, 1);
    connection.execute_batch("DROP TRIGGER reject_event")?;
    store.finish(&task_id, 1, &verified())?;
    assert_eq!(store.events(&task_id, 0)?.len(), 2);
    drop(connection);
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn competing_writer_times_out_without_partial_record() -> Result<(), Box<dyn std::error::Error>> {
    let path = database_path();
    let mut store = TaskStore::open(&path)?;
    let connection = Connection::open(&path)?;
    connection.execute_batch("BEGIN IMMEDIATE")?;
    let start = Instant::now();
    assert!(matches!(store.start(), Err(StoreError::Database(_))));
    assert!(start.elapsed().as_millis() >= 400);
    assert!(start.elapsed().as_secs_f64() < 3.0);
    connection.execute_batch("ROLLBACK")?;
    let count: i64 = connection.query_row("SELECT count(*) FROM tasks", [], |r| r.get(0))?;
    assert_eq!(count, 0);
    let id = store.start()?.0;
    drop(store);
    let reopened = TaskStore::open(&path)?;
    assert_eq!(reopened.status(&id)?.recorded_state, "running");
    assert_eq!(reopened.events(&id, 0)?.len(), 1);
    drop(reopened);
    drop(connection);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn read_only_connection_never_creates_or_changes_task_data()
-> Result<(), Box<dyn std::error::Error>> {
    let path = database_path();
    assert!(TaskStore::read(&path).is_err());
    assert!(!path.exists());
    let mut writer = TaskStore::open(&path)?;
    let id = writer.start()?.0;
    assert!(matches!(
        writer.finish(&id, 0, &verified()),
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        writer.finish("missing", 1, &verified()),
        Err(StoreError::NotFound)
    ));
    let mut reader = TaskStore::read(&path)?;
    assert!(reader.start().is_err());
    assert!(reader.finish(&id, 1, &verified()).is_err());
    assert_eq!(reader.status(&id)?.recorded_state, "running");
    writer.finish(&id, 1, &verified())?;
    assert_eq!(reader.status(&id)?.recorded_state, "succeeded");
    drop(reader);
    drop(writer);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn event_pages_are_bounded_and_use_the_task_sequence_index()
-> Result<(), Box<dyn std::error::Error>> {
    let path = database_path();
    let mut store = TaskStore::open(&path)?;
    let id = store.start()?.0;
    let connection = Connection::open(&path)?;
    let version: String = connection.query_row("SELECT sqlite_version()", [], |r| r.get(0))?;
    assert_eq!(version, rusqlite::version());
    assert!(rusqlite::version_number() >= 3_051_003);
    println!("SQLite runtime: {version}");
    connection.execute(
        "WITH RECURSIVE n(seq) AS (SELECT 2 UNION ALL SELECT seq+1 FROM n WHERE seq<250)
         INSERT INTO events(event_id,task_id,seq,event_type) SELECT CAST(seq AS TEXT),?1,seq,'started' FROM n", [&id])?;
    let first = store.events(&id, 0)?;
    let second = store.events(&id, 100)?;
    let third = store.events(&id, 200)?;
    assert_eq!(first.len(), 100);
    assert_eq!(second.len(), 100);
    assert_eq!(third.len(), 50);
    let sequences: Vec<_> = first
        .iter()
        .chain(&second)
        .chain(&third)
        .map(|event| event.seq)
        .collect();
    assert_eq!(sequences, (1..=250).collect::<Vec<_>>());
    let plan: String = connection.query_row(
        "EXPLAIN QUERY PLAN SELECT * FROM events WHERE task_id=?1 AND seq>?2 ORDER BY seq LIMIT 100",
        rusqlite::params![id, 100], |r| r.get(3))?;
    assert!(plan.contains("SEARCH events USING INDEX"), "{plan}");
    assert!(!plan.contains("SCAN"), "{plan}");
    drop(connection);
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn concurrent_results_cannot_overwrite_each_other() -> Result<(), Box<dyn std::error::Error>> {
    let path = database_path();
    let mut store = TaskStore::open(&path)?;
    let id = store.start()?.0;
    let competitor = TaskStore::open(&path)?;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let other_barrier = barrier.clone();
    let other_id = id.clone();
    let worker = std::thread::spawn(move || {
        let mut competitor = competitor;
        other_barrier.wait();
        competitor.finish(&other_id, 1, &Err(TaskError::MissingArtifact))
    });
    barrier.wait();
    let first = store.finish(&id, 1, &verified());
    let second = worker.join().expect("worker must finish");
    assert!(matches!(
        (&first, &second),
        (Ok(()), Err(StoreError::Conflict)) | (Err(StoreError::Conflict), Ok(()))
    ));
    let state = if first.is_ok() { "succeeded" } else { "failed" };
    assert_eq!(store.status(&id)?.recorded_state, state);
    let events = store.events(&id, 0)?;
    assert_eq!(events.len(), 2);
    assert_eq!(events[1].event_type, state);
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn read_only_query_survives_exit_before_wal_configuration() -> Result<(), Box<dyn std::error::Error>>
{
    let path = database_path();
    let mut store = TaskStore::open(&path)?;
    let id = store.start()?.0;
    drop(store);
    let connection = Connection::open(&path)?;
    connection.pragma_update(None, "journal_mode", "delete")?;
    drop(connection);
    let reader = TaskStore::read(&path)?;
    assert_eq!(reader.status(&id)?.recorded_state, "running");
    assert_eq!(reader.events(&id, 0)?.len(), 1);
    drop(reader);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn foreign_and_newer_databases_are_not_migrated() -> Result<(), Box<dyn std::error::Error>> {
    for setup in [
        "PRAGMA user_version=4",
        "PRAGMA application_id=42",
        "CREATE TABLE unrelated (value TEXT)",
    ] {
        let path = database_path();
        let connection = Connection::open(&path)?;
        connection.execute_batch(setup)?;
        assert!(matches!(
            TaskStore::open(&path),
            Err(StoreError::Incompatible)
        ));
        let tasks: i64 = connection.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name='tasks'",
            [],
            |r| r.get(0),
        )?;
        assert_eq!(tasks, 0);
        let mode: String = connection.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
        assert_eq!(mode, "delete");
        drop(connection);
        std::fs::remove_file(path)?;
    }
    Ok(())
}
