use agent_harness::{
    Agent, CancellationToken, Message, Model, ModelError, ModelReply, ToolSpec, TurnStatus,
};
use agent_platform_agent::{
    store::{INSTRUCTION_PENDING_LIMIT, StoreError, TaskStore},
    task_session::TaskSession,
};
use std::sync::{Arc, Mutex};

struct Scripted {
    store: Arc<Mutex<TaskStore>>,
    task: String,
    cancel: Option<CancellationToken>,
    seen: Arc<Mutex<Vec<Vec<Message>>>>,
}

impl Model for Scripted {
    async fn complete(
        &self,
        messages: &[Message],
        _: &[ToolSpec],
    ) -> Result<ModelReply, ModelError> {
        let turn = {
            let mut seen = self.seen.lock().map_err(|_| ModelError("lock".into()))?;
            seen.push(messages.to_vec());
            seen.len()
        };
        if turn == 1 {
            self.store
                .lock()
                .map_err(|_| ModelError("lock".into()))?
                .receive_instruction(&self.task, "m-1", "改用繁體中文")
                .map_err(|error| ModelError(error.to_string()))?;
            if let Some(cancel) = &self.cancel {
                cancel.cancel();
            }
        }
        let status = if turn == 1 {
            TurnStatus::Continue
        } else {
            TurnStatus::Complete
        };
        Ok(ModelReply {
            message: Message::text("ok"),
            status,
        })
    }
}

fn instruction_status(path: &std::path::Path) -> Result<String, Box<dyn std::error::Error>> {
    Ok(rusqlite::Connection::open(path)?.query_row(
        "SELECT status FROM instructions",
        [],
        |row| row.get(0),
    )?)
}

async fn run(cancelled: bool) -> Result<(Vec<Vec<Message>>, String), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!(
        "agent-instruction-apply-{}.sqlite3",
        uuid::Uuid::new_v4()
    ));
    let mut store = TaskStore::open(&path)?;
    let (task, generation) = store.start()?;
    let store = Arc::new(Mutex::new(store));
    let cancel = CancellationToken::new();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let model = Scripted {
        store: store.clone(),
        task: task.clone(),
        cancel: cancelled.then(|| cancel.clone()),
        seen: seen.clone(),
    };
    let session = TaskSession::new(store.clone(), task.clone(), generation)?;
    let agent = Agent::new(model).session(session);
    let _ = agent.run("work", &cancel).await;
    let status = instruction_status(&path)?;
    let seen = seen.lock().map_err(|_| "lock")?.clone();
    drop(agent);
    drop(store);
    std::fs::remove_file(path)?;
    Ok((seen, status))
}

#[tokio::test]
async fn received_instruction_reaches_the_model_at_the_next_turn_and_is_marked_applied()
-> Result<(), Box<dyn std::error::Error>> {
    let (seen, status) = run(false).await?;
    assert_eq!(seen.len(), 2);
    assert!(!seen[0].contains(&Message::User("補充指令 1：改用繁體中文".into())));
    assert!(seen[1].contains(&Message::User("補充指令 1：改用繁體中文".into())));
    assert_eq!(status, "applied");
    Ok(())
}

#[tokio::test]
async fn cancellation_wins_over_a_pending_instruction() -> Result<(), Box<dyn std::error::Error>> {
    let (seen, status) = run(true).await?;
    assert_eq!(seen.len(), 1);
    assert_eq!(status, "received");
    Ok(())
}

fn full_backlog() -> Result<(TaskStore, String, std::path::PathBuf), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!(
        "agent-instruction-backlog-{}.sqlite3",
        uuid::Uuid::new_v4()
    ));
    let mut store = TaskStore::open(&path)?;
    let (task, _) = store.start()?;
    for index in 0..INSTRUCTION_PENDING_LIMIT {
        store.receive_instruction(&task, &format!("m-{index}"), "x")?;
    }
    Ok((store, task, path))
}

#[test]
fn the_pending_limit_is_exact_and_a_resend_still_succeeds() -> Result<(), Box<dyn std::error::Error>>
{
    let (mut store, task, path) = full_backlog()?;
    assert!(matches!(
        store.receive_instruction(&task, "over", "x"),
        Err(StoreError::InstructionBacklog)
    ));
    assert!(store.receive_instruction(&task, "m-0", "x")?.duplicate);
    assert_eq!(
        store.status(&task)?.pending_instructions,
        INSTRUCTION_PENDING_LIMIT
    );
    rusqlite::Connection::open(&path)?
        .execute("UPDATE instructions SET status='applied' WHERE seq=1", [])?;
    assert_eq!(
        store.status(&task)?.pending_instructions,
        INSTRUCTION_PENDING_LIMIT - 1
    );
    assert_eq!(
        store.receive_instruction(&task, "over", "x")?.seq,
        INSTRUCTION_PENDING_LIMIT + 1
    );
    drop(store);
    std::fs::remove_file(path)?;
    Ok(())
}
