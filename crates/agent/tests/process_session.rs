use agent_harness::{
    Agent, CancellationToken, Dispatch, DispatchAction, Hook, Message, Model, ModelError,
    ModelReply, RunError, SessionError, SessionStore, Tool, ToolFuture, ToolSpec, TurnStatus,
};
use agent_platform_agent::{store::TaskStore, task_session::TaskSession};
use serde_json::{Value, json};
use std::{
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

type Failure = Box<dyn std::error::Error>;

struct ModelScript;
impl Model for ModelScript {
    async fn complete(
        &self,
        messages: &[Message],
        _: &[ToolSpec],
    ) -> Result<ModelReply, ModelError> {
        let called = messages.iter().any(|m| matches!(m, Message::Tool { .. }));
        Ok(ModelReply {
            message: if called {
                Message::text("done")
            } else {
                Message::call("call", "effect", json!({}))
            },
            status: if called {
                TurnStatus::Complete
            } else {
                TurnStatus::ToolCalls
            },
        })
    }
}

struct Boundary {
    root: PathBuf,
    before: bool,
}
impl Hook for Boundary {
    async fn on_dispatch(&self, _: Dispatch<'_>) -> DispatchAction {
        if self.before {
            std::fs::write(self.root.join("ready"), "before").unwrap();
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
        DispatchAction::Proceed
    }
}
struct Effect {
    root: PathBuf,
    hold: bool,
}
impl Tool for Effect {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "effect".into(),
            description: "test effect".into(),
            parameters: json!({"type":"object"}),
        }
    }
    fn call(&self, _: Value) -> ToolFuture<'_> {
        Box::pin(async {
            let mut file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.root.join("effect"))
                .unwrap();
            file.write_all(b"effect\n").unwrap();
            file.sync_all().unwrap();
            if self.hold {
                std::fs::write(self.root.join("ready"), "after").unwrap();
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
            Ok("effect".into())
        })
    }
}

#[test]
fn subprocess_worker() -> Result<(), Failure> {
    let Some(root) = std::env::var_os("SESSION_PROCESS_ROOT") else {
        return Ok(());
    };
    let root = PathBuf::from(root);
    let task = std::env::var("SESSION_PROCESS_TASK")?;
    let mode = std::env::var("SESSION_PROCESS_MODE")?;
    let store = Arc::new(Mutex::new(TaskStore::open(&root.join("tasks.sqlite3"))?));
    let session = TaskSession::new(store, task, 1);
    if mode == "contender" {
        assert!(matches!(session, Err(SessionError::Conflict)));
        println!("ownership_conflict");
        return Ok(());
    }
    let session = session?;
    let prior = session.load()?;
    if mode == "recover" {
        let saved = prior.as_ref().unwrap();
        assert_eq!(
            saved.messages.len(),
            if saved.unresolved_effect { 3 } else { 2 }
        );
        assert!(!saved.completed);
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    let result = runtime.block_on(
        Agent::new(ModelScript)
            .session(&session)
            .hook(Boundary {
                root: root.clone(),
                before: mode == "before",
            })
            .tool(Effect {
                root,
                hold: mode == "after",
            })
            .run("work", &CancellationToken::new()),
    );
    if mode == "recover" {
        assert!(matches!(
            result,
            Err(RunError::Session(SessionError::Unsettled))
        ));
        let saved = session.load()?.unwrap();
        assert!(saved.unresolved_effect);
        assert_eq!(saved.messages.len(), 3);
        assert_eq!(
            saved.messages[2],
            Message::Tool {
                call_id: "call".into(),
                content: "結果未知".into(),
                is_error: true,
            }
        );
        if prior.as_ref().unwrap().unresolved_effect {
            assert_eq!(Some(saved), prior);
        }
        println!("recovery_refused_unknown");
        return Ok(());
    }
    result?;
    Ok(())
}

struct Worker(Child);
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn command(root: &Path, task: &str, mode: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "subprocess_worker", "--nocapture"])
        .env("SESSION_PROCESS_ROOT", root)
        .env("SESSION_PROCESS_TASK", task)
        .env("SESSION_PROCESS_MODE", mode)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}
fn checked(root: &Path, task: &str, mode: &str, expected: &str) {
    let output = command(root, task, mode).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains(expected));
}

#[test]
fn real_process_ownership_and_kill_boundaries_do_not_repeat_effects() -> Result<(), Failure> {
    for mode in ["before", "after"] {
        let root =
            std::env::temp_dir().join(format!("agent-process-session-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root)?;
        let mut store = TaskStore::open(&root.join("tasks.sqlite3"))?;
        let (task, _) = store.start()?;
        drop(store);
        let mut owner = Worker(command(&root, &task, mode).spawn()?);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !root.join("ready").exists() && Instant::now() < deadline {
            assert!(
                owner.0.try_wait()?.is_none(),
                "worker ended before boundary"
            );
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(root.join("ready").exists(), "boundary timeout");
        checked(&root, &task, "contender", "ownership_conflict");
        owner.0.kill()?;
        let exit = owner.0.wait()?;
        assert!(!exit.success());
        checked(&root, &task, "recover", "recovery_refused_unknown");
        checked(&root, &task, "recover", "recovery_refused_unknown");
        let effect = root.join("effect");
        if mode == "before" {
            assert!(!effect.exists());
        }
        if mode == "after" {
            assert_eq!(std::fs::read_to_string(effect)?, "effect\n");
        }
        std::fs::remove_dir_all(root)?;
    }
    Ok(())
}
