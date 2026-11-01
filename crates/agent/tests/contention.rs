use agent_platform_agent::{
    store::{ApprovalMode, Claim, TaskSpec, TaskStore},
    task::{FileTask, TaskError},
};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};

type Failure = Box<dyn std::error::Error>;

fn spec() -> TaskSpec {
    let workspace = std::env::temp_dir();
    TaskSpec {
        task: FileTask {
            prompt: "work".into(),
            input: workspace.join("input"),
            output: workspace.join("output"),
            expected: Some("result".into()),
            allow_read: true,
            allow_write: true,
        },
        workspace,
        endpoint: "http://127.0.0.1:1/v1".into(),
        model: "fixture-model".into(),
        approval_mode: ApprovalMode::Manual,
    }
}

fn path() -> PathBuf {
    std::env::temp_dir().join(format!("agent-contention-{}.sqlite3", uuid::Uuid::new_v4()))
}

fn approved(path: &Path) -> Result<(String, agent_platform_agent::store::PreparedIntent), Failure> {
    let mut store = TaskStore::open(path)?;
    let (id, _) = store.start_spec(&spec())?;
    let prepared =
        store.prepare_intent(&id, 0, "call-1", "write_output", r#"{"content":"result"}"#)?;
    store.decide(&prepared.approval_id, 1, true)?;
    Ok((id, prepared))
}

#[test]
fn two_connections_claiming_one_approval_dispatch_it_once() -> Result<(), Failure> {
    for _ in 0..10 {
        let path = path();
        let (_, prepared) = approved(&path)?;
        let prepared = Arc::new(prepared);
        let barrier = Arc::new(Barrier::new(2));
        let workers: Vec<_> = (0..2)
            .map(|_| {
                let (path, prepared, barrier) = (path.clone(), prepared.clone(), barrier.clone());
                std::thread::spawn(move || {
                    let mut store = TaskStore::open(&path).ok()?;
                    barrier.wait();
                    store.claim(&prepared).ok()
                })
            })
            .collect();
        let results: Vec<Option<Claim>> = workers
            .into_iter()
            .map(|worker| worker.join().ok().flatten())
            .collect();
        let granted = results
            .iter()
            .filter(|r| **r == Some(Claim::Granted))
            .count();
        assert_eq!(granted, 1, "{results:?}");
        std::fs::remove_file(path)?;
    }
    Ok(())
}

#[test]
fn dropped_connection_after_dispatch_is_not_dispatched_again_and_finish_marks_unknown()
-> Result<(), Failure> {
    let path = path();
    let (id, prepared) = approved(&path)?;
    let mut crashed = TaskStore::open(&path)?;
    assert_eq!(crashed.claim(&prepared)?, Claim::Granted);
    drop(crashed);
    let mut restarted = TaskStore::open(&path)?;
    assert_eq!(restarted.claim(&prepared)?, Claim::Denied);
    let generation = restarted.status(&id)?.generation;
    restarted.finish(&id, generation, &Err(TaskError::State))?;
    assert_eq!(restarted.status(&id)?.recorded_state, "outcome_unknown");
    let kinds: Vec<String> = restarted
        .events(&id, 0)?
        .into_iter()
        .map(|e| e.event_type)
        .collect();
    assert!(
        kinds.contains(&"tool_outcome_unknown".to_string()),
        "{kinds:?}"
    );
    std::fs::remove_file(path)?;
    Ok(())
}

#[test]
fn dropped_connection_before_dispatch_allows_exactly_one_later_dispatch() -> Result<(), Failure> {
    let path = path();
    let (_, prepared) = approved(&path)?;
    let mut restarted = TaskStore::open(&path)?;
    assert_eq!(restarted.claim(&prepared)?, Claim::Granted);
    assert_eq!(restarted.claim(&prepared)?, Claim::Denied);
    std::fs::remove_file(path)?;
    Ok(())
}
