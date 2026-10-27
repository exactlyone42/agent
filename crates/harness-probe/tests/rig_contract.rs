use agent_platform_agent::verification::verify_file;
use rig_agent::{
    AgentBuilder,
    agent::{AgentHook, DispatchAction, DispatchEvent, HookContext},
    run::PromptError,
    tool::{Tool, ToolContext, ToolExecutionError},
};
use rig_core::{
    completion::message::{Message, ToolResultContent, UserContent},
    effect::EffectKind,
    test_utils::{MockCompletionModel, MockTurn},
};
use serde::Deserialize;
use serde_json::json;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    path: String,
}

struct ReadFixture(Arc<AtomicUsize>, std::path::PathBuf);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WriteArgs {
    content: String,
}

struct WriteFixture(std::path::PathBuf, Arc<Mutex<Option<std::fs::File>>>);

impl Tool for WriteFixture {
    const NAME: &'static str = "write_fixture";
    type Args = WriteArgs;
    type Output = String;
    type Error = ToolExecutionError;

    fn description(&self) -> String {
        "寫入合成測試成果".into()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"content":{"type":"string"}},"required":["content"],"additionalProperties":false})
    }

    async fn call(&self, _ctx: &mut ToolContext, args: WriteArgs) -> Result<String, Self::Error> {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&self.0)
            .map_err(ToolExecutionError::from_error)?;
        std::io::Write::write_all(&mut file, args.content.as_bytes())
            .map_err(ToolExecutionError::from_error)?;
        *self
            .1
            .lock()
            .map_err(|_| ToolExecutionError::refused("成果狀態已損壞"))? = Some(file);
        Ok("寫入完成，仍須獨立讀回驗證".into())
    }
}

fn fixture() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let path = std::env::temp_dir().join(format!("agent-platform-{}-{stamp}", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    std::io::Write::write_all(&mut file, b"original fixture")?;
    Ok(path)
}

impl Tool for ReadFixture {
    const NAME: &'static str = "read_fixture";
    type Args = Args;
    type Output = String;
    type Error = ToolExecutionError;

    fn description(&self) -> String {
        "讀取合成測試資料".into()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({"type":"object","properties":{"path":{"type":"string"}},"required":["path"],"additionalProperties":false})
    }

    async fn call(&self, _ctx: &mut ToolContext, args: Args) -> Result<String, Self::Error> {
        if args.path != "README.md" {
            return Err(ToolExecutionError::refused("未授權的檔案路徑"));
        }
        self.0.fetch_add(1, Ordering::SeqCst);
        std::fs::read_to_string(&self.1).map_err(ToolExecutionError::from_error)
    }
}

struct Approval(bool);

impl AgentHook for Approval {
    async fn on_dispatch(&self, _ctx: &HookContext, event: DispatchEvent<'_>) -> DispatchAction {
        if let EffectKind::ToolCall { name, args, .. } = event.kind {
            if !self.0 {
                return DispatchAction::stop("使用者拒絕工具操作");
            }
            let valid = match name.as_str() {
                "read_fixture" => serde_json::from_str::<Args>(args).is_ok(),
                "write_fixture" => serde_json::from_str::<WriteArgs>(args).is_ok(),
                _ => false,
            };
            if !valid {
                return DispatchAction::stop("工具參數格式錯誤");
            }
        }
        DispatchAction::proceed()
    }
}

#[tokio::test]
async fn approval_controls_actual_tool_execution() -> Result<(), Box<dyn std::error::Error>> {
    for allowed in [false, true] {
        let file = fixture()?;
        std::fs::write(&file, "updated fixture content")?;
        let calls = Arc::new(AtomicUsize::new(0));
        let model = MockCompletionModel::from_turns([
            MockTurn::tool_call("tc-1", "read_fixture", json!({"path":"README.md"})),
            MockTurn::text("done"),
        ]);
        let agent = AgentBuilder::new(model.clone())
            .tool(ReadFixture(calls.clone(), file.clone()))
            .add_hook(Approval(allowed))
            .build();
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            agent.prompt("go").max_turns(3).run(),
        )
        .await?;
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(allowed));
        if allowed {
            assert_eq!(result?.output(), "done");
            assert_eq!(model.request_count(), 2);
            let requests = format!("{:?}", model.requests());
            assert!(requests.contains("updated fixture content"), "{requests}");
        } else {
            assert!(
                matches!(result, Err(PromptError::Cancelled { reason, .. }) if reason == "使用者拒絕工具操作")
            );
            assert_eq!(model.request_count(), 1);
        }
        std::fs::remove_file(file)?;
    }
    Ok(())
}

#[tokio::test]
async fn malformed_arguments_do_not_execute_tool() -> Result<(), Box<dyn std::error::Error>> {
    let file = fixture()?;
    let calls = Arc::new(AtomicUsize::new(0));
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call("tc-1", "read_fixture", json!({"path":42})),
        MockTurn::text("done"),
    ]);
    let agent = AgentBuilder::new(model.clone())
        .tool(ReadFixture(calls.clone(), file.clone()))
        .add_hook(Approval(true))
        .build();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        agent.prompt("go").max_turns(3).run(),
    )
    .await?;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(
        matches!(result, Err(PromptError::Cancelled { reason, .. }) if reason == "工具參數格式錯誤")
    );
    assert_eq!(model.request_count(), 1);
    std::fs::remove_file(file)?;
    Ok(())
}

#[tokio::test]
async fn unauthorized_path_does_not_read_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let file = fixture()?;
    let calls = Arc::new(AtomicUsize::new(0));
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call("tc-1", "read_fixture", json!({"path":"../secret.txt"})),
        MockTurn::text("done"),
    ]);
    let agent = AgentBuilder::new(model.clone())
        .tool(ReadFixture(calls.clone(), file.clone()))
        .add_hook(Approval(true))
        .build();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        agent.prompt("go").max_turns(3).run(),
    )
    .await?;
    assert!(result.is_ok());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let requests = format!("{:?}", model.requests());
    assert!(!requests.contains("original fixture"));
    assert!(requests.contains("未授權的檔案路徑"));
    assert!(model.requests().iter().flat_map(|r| r.chat_history.iter()).any(|m| {
        matches!(m.as_ref(), Message::User { content } if content.iter().any(|c| matches!(c, UserContent::ToolResult(r) if r.is_error)))
    }));
    std::fs::remove_file(file)?;
    Ok(())
}

#[tokio::test]
async fn missing_file_is_a_tool_error_not_content() -> Result<(), Box<dyn std::error::Error>> {
    let file = fixture()?;
    std::fs::remove_file(&file)?;
    let calls = Arc::new(AtomicUsize::new(0));
    let model = MockCompletionModel::from_turns([
        MockTurn::tool_call("tc-1", "read_fixture", json!({"path":"README.md"})),
        MockTurn::text("done"),
    ]);
    let agent = AgentBuilder::new(model.clone())
        .tool(ReadFixture(calls.clone(), file))
        .add_hook(Approval(true))
        .build();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        agent.prompt("go").max_turns(3).run(),
    )
    .await??;
    assert_eq!(result.output(), "done");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let requests = model.requests();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].chat_history.iter().any(|m| {
        matches!(m.as_ref(), Message::User { content } if content.iter().any(|c| matches!(c, UserContent::ToolResult(r) if r.is_error)))
    }));
    Ok(())
}

#[tokio::test]
async fn artifact_readback_distinguishes_completion_from_model_claims()
-> Result<(), Box<dyn std::error::Error>> {
    for (allowed, content, expected) in [
        (true, Some("ORIGINAL FIXTURE"), true),
        (true, Some("incorrect content"), false),
        (true, None, false),
        (false, Some("ORIGINAL FIXTURE"), false),
    ] {
        let source = fixture()?;
        let destination = source.with_extension("result");
        let artifact = Arc::new(Mutex::new(None));
        let calls = Arc::new(AtomicUsize::new(0));
        let mut turns = vec![MockTurn::tool_call(
            "read-1",
            "read_fixture",
            json!({"path":"README.md"}),
        )];
        if let Some(content) = content {
            turns.push(MockTurn::tool_call(
                "write-1",
                "write_fixture",
                json!({"content":content}),
            ));
        }
        turns.push(MockTurn::text("done"));
        let model = MockCompletionModel::from_turns(turns);
        let agent = AgentBuilder::new(model.clone())
            .tool(ReadFixture(calls.clone(), source.clone()))
            .tool(WriteFixture(destination.clone(), artifact.clone()))
            .add_hook(Approval(allowed))
            .build();
        let result = tokio::time::timeout(
            Duration::from_secs(5),
            agent
                .prompt("讀取 README.md，將大寫內容寫入成果檔")
                .max_turns(3)
                .run(),
        )
        .await?;
        let verified = artifact
            .lock()
            .map_err(|_| "成果狀態已損壞")?
            .as_mut()
            .map(|file| verify_file(file, b"ORIGINAL FIXTURE"));
        assert_eq!(verified.as_ref().is_some_and(Result::is_ok), expected);
        assert_eq!(calls.load(Ordering::SeqCst), usize::from(allowed));
        if allowed {
            assert_eq!(result?.output(), "done");
            let requests = model.requests();
            assert!(requests[1].chat_history.iter().any(|m| {
                matches!(m.as_ref(), Message::User { content } if content.iter().any(|c| matches!(c, UserContent::ToolResult(r) if !r.is_error && r.content.iter().any(|v| matches!(v, ToolResultContent::Text(t) if t.text == "original fixture")))))
            }));
        } else {
            assert!(
                matches!(result, Err(PromptError::Cancelled { reason, .. }) if reason == "使用者拒絕工具操作")
            );
            assert!(verified.is_none());
        }
        drop(agent);
        drop(artifact);
        if destination.try_exists()? {
            std::fs::remove_file(destination)?;
        }
        std::fs::remove_file(source)?;
    }
    Ok(())
}
