#[path = "session/v3_upgrade.rs"]
mod v3_upgrade;

#[path = "session/v4_upgrade.rs"]
mod v4_upgrade;

use agent_harness::{Message, Session, SessionError, SessionStore};
use agent_platform_agent::{store::TaskStore, task::TaskError, task_session::TaskSession};
use std::sync::{Arc, Mutex};

#[test]
fn sqlite_session_size_limit_is_exact_and_failure_preserves_prior_history()
-> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("agent-size-{}.sqlite3", uuid::Uuid::new_v4()));
    let mut store = TaskStore::open(&path)?;
    let (task, generation) = store.start()?;
    let shared = Arc::new(Mutex::new(store));
    let session_store = TaskSession::new(shared.clone(), task.clone(), generation)?;
    let mut session = Session {
        revision: 1,
        messages: vec![Message::User(String::new())],
        next_turn: 2,
        completed: false,
        unresolved_effect: true,
    };
    let overhead = serde_json::to_vec(&session)?.len();
    session.messages = vec![Message::User("a".repeat(32 * 1024 * 1024 - overhead))];
    assert_eq!(serde_json::to_vec(&session)?.len(), 32 * 1024 * 1024);
    session.revision = 0;
    session.revision = session_store.save(&session)?;
    let saved = session.clone();
    if let Message::User(text) = &mut session.messages[0] {
        text.push('b');
    }
    assert!(matches!(
        session_store.save(&session),
        Err(SessionError::Storage)
    ));
    assert_eq!(session_store.load()?.unwrap(), saved);
    drop(session_store);
    let reopened = TaskSession::new(shared, task, generation)?;
    assert_eq!(reopened.load()?.unwrap(), saved);
    drop(reopened);
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn sqlite_history_survives_reopen_and_rejects_stale_or_finished_writers()
-> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("agent-session-{}.sqlite3", uuid::Uuid::new_v4()));
    let mut store = TaskStore::open(&path)?;
    let (task, generation) = store.start()?;
    let shared = Arc::new(Mutex::new(store));
    let session_store = TaskSession::new(shared.clone(), task.clone(), generation)?;
    assert!(session_store.load()?.is_none());
    let mut session = Session {
        revision: 0,
        messages: vec![Message::User("work".into())],
        next_turn: 0,
        completed: false,
        unresolved_effect: false,
    };
    session.revision = session_store.save(&session)?;
    let stale = session.clone();
    session.messages.push(Message::text("finished"));
    session.completed = true;
    session.revision = session_store.save(&session)?;
    assert!(matches!(
        session_store.save(&stale),
        Err(SessionError::Conflict)
    ));
    drop(session_store);
    drop(shared);
    let shared = Arc::new(Mutex::new(TaskStore::open(&path)?));
    let reopened = TaskSession::new(shared.clone(), task.clone(), generation)?;
    assert_eq!(reopened.load()?.unwrap(), session);
    assert!(matches!(
        TaskSession::new(shared.clone(), task.clone(), generation + 1),
        Err(SessionError::Conflict)
    ));
    shared
        .lock()
        .unwrap()
        .finish(&task, generation, &Err(TaskError::MissingArtifact))?;
    assert!(matches!(
        reopened.save(&session),
        Err(SessionError::Conflict)
    ));
    drop(reopened);
    drop(shared);
    std::fs::remove_file(path)?;
    Ok(())
}
