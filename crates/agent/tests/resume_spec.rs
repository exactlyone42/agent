use agent_harness::{Message, Session, SessionStore};
use agent_platform_agent::{
    store::{ApprovalMode, StoreError, TaskSpec, TaskStore},
    task::FileTask,
    task_session::TaskSession,
};
use std::sync::{Arc, Mutex};

#[test]
fn mismatched_resume_spec_preserves_history_and_releases_ownership()
-> Result<(), Box<dyn std::error::Error>> {
    let directory =
        std::env::temp_dir().join(format!("agent-resume-spec-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
    }
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
            allow_read: true,
            allow_write: true,
        },
    };
    let mut store = TaskStore::open(&directory.join("tasks.sqlite3"))?;
    let (task, generation) = store.start_spec(&spec)?;
    let shared = Arc::new(Mutex::new(store));
    let original = TaskSession::new(shared.clone(), task.clone(), generation)?;
    let mut saved = Session {
        revision: 0,
        messages: vec![Message::User("work".into())],
        next_turn: 1,
        completed: false,
        unresolved_effect: false,
    };
    saved.revision = original.save(&saved)?;
    drop(original);
    let before = shared.lock().unwrap().status(&task)?;
    let mut mismatched = spec.clone();
    mismatched.task.prompt = "different work".into();
    assert!(matches!(
        TaskSession::resume(shared.clone(), task.clone(), &mismatched),
        Err(StoreError::Conflict)
    ));
    let after = shared.lock().unwrap().status(&task)?;
    assert_eq!(
        (after.generation, after.last_seq, after.recorded_state),
        (before.generation, before.last_seq, before.recorded_state)
    );
    let reopened = TaskSession::new(shared.clone(), task.clone(), generation)?;
    assert_eq!(reopened.load()?.unwrap(), saved);
    drop(reopened);
    let (resumed, artifact) = TaskSession::resume(shared.clone(), task, &spec)?;
    assert_eq!(resumed.generation(), generation + 1);
    assert!(artifact.is_none());
    assert!(
        resumed
            .load()?
            .unwrap()
            .messages
            .starts_with(&saved.messages)
    );
    drop(resumed);
    drop(shared);
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
