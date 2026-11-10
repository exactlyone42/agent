use agent_harness::{Message, Session, ToolCall};
use agent_platform_agent::{
    result::{ResultError, TaskResult},
    store::{ApprovalMode, ResultSnapshot, SavedToolCall, TaskSpec, TaskStore},
    task::FileTask,
};
use serde_json::json;
use std::path::{Path, PathBuf};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const FILE_LIMIT: usize = 1024 * 1024;

fn root() -> Result<PathBuf, std::io::Error> {
    let root = std::env::temp_dir().join(format!("agent-inspect-limits-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root)?;
    Ok(root)
}

fn snapshot(
    root: &Path,
    output: &Path,
    expected: Option<&str>,
) -> Result<ResultSnapshot, Box<dyn std::error::Error>> {
    let mut store = TaskStore::open(&root.join("tasks.sqlite3"))?;
    let spec = TaskSpec {
        workspace: root.to_path_buf(),
        endpoint: "http://127.0.0.1/v1".into(),
        model: "test".into(),
        approval_mode: ApprovalMode::Preapproved,
        task: FileTask {
            prompt: "work".into(),
            input: root.join("input"),
            output: output.to_path_buf(),
            expected: expected.map(str::to_owned),
            allow_read: true,
            allow_write: true,
        },
    };
    let (task, _) = store.start_spec(&spec)?;
    Ok(store.result_snapshot(&task)?)
}

fn tool_call(id: &str, name: &str, args: serde_json::Value, bad: Option<&str>) -> Message {
    Message::Assistant {
        reasoning: None,
        text: String::new(),
        calls: vec![ToolCall {
            id: id.into(),
            name: name.into(),
            args,
            bad_args: bad.map(str::to_owned),
        }],
    }
}

fn tool_result(id: &str, content: &str) -> Message {
    Message::Tool {
        call_id: id.into(),
        content: content.into(),
        is_error: false,
    }
}

fn saved(id: &str, name: &str, args: &serde_json::Value) -> SavedToolCall {
    SavedToolCall {
        call_id: id.into(),
        tool_name: name.into(),
        args_json: args.to_string(),
    }
}

fn session(messages: Vec<Message>, completed: bool, unresolved_effect: bool) -> Session {
    Session {
        revision: 1,
        messages,
        next_turn: 0,
        completed,
        unresolved_effect,
    }
}

fn written(root: &Path, content: &str) -> Result<ResultSnapshot, Box<dyn std::error::Error>> {
    let output = root.join("output");
    std::fs::write(&output, content)?;
    let args = json!({"content": content});
    let mut snapshot = snapshot(root, &output, Some(content))?;
    snapshot.record.recorded_state = "succeeded".into();
    snapshot.session = Some(session(
        vec![
            tool_call("w1", "write_output", args.clone(), None),
            tool_result("w1", "ok"),
        ],
        true,
        false,
    ));
    snapshot.successful_calls = vec![saved("w1", "write_output", &args)];
    Ok(snapshot)
}

#[test]
fn verified_needs_success_state_matching_content_and_a_settled_session() -> TestResult {
    let root = root()?;
    assert!(TaskResult::inspect(written(&root, "data")?)?.verified);

    let mut failed = written(&root, "data")?;
    failed.record.recorded_state = "failed".into();
    assert!(!TaskResult::inspect(failed)?.verified);

    let mut unresolved = written(&root, "data")?;
    if let Some(session) = unresolved.session.as_mut() {
        session.unresolved_effect = true;
    }
    assert!(!TaskResult::inspect(unresolved)?.verified);

    let mut open = written(&root, "data")?;
    if let Some(session) = open.session.as_mut() {
        session.completed = false;
        session.unresolved_effect = false;
    }
    assert!(!TaskResult::inspect(open)?.verified);
    Ok(())
}

#[test]
fn repeated_writes_must_agree() -> TestResult {
    let root = root()?;
    let mut same = written(&root, "data")?;
    let args = json!({"content": "data"});
    if let Some(session) = same.session.as_mut() {
        session
            .messages
            .push(tool_call("w2", "write_output", args.clone(), None));
        session.messages.push(tool_result("w2", "ok"));
    }
    same.successful_calls
        .push(saved("w2", "write_output", &args));
    assert!(TaskResult::inspect(same)?.verified);

    let mut different = written(&root, "data")?;
    let other = json!({"content": "other"});
    if let Some(session) = different.session.as_mut() {
        session
            .messages
            .push(tool_call("w2", "write_output", other.clone(), None));
        session.messages.push(tool_result("w2", "ok"));
    }
    different
        .successful_calls
        .push(saved("w2", "write_output", &other));
    assert!(matches!(
        TaskResult::inspect(different),
        Err(ResultError::InvalidHistory)
    ));
    Ok(())
}

#[test]
fn saved_call_must_match_the_recorded_assistant_call() -> TestResult {
    let root = root()?;
    let args = json!({"content": "data"});
    let cases = [
        ("name", tool_call("w1", "read_input", args.clone(), None)),
        (
            "args",
            tool_call("w1", "write_output", json!({"content": "zzz"}), None),
        ),
        (
            "bad",
            tool_call("w1", "write_output", args.clone(), Some("broken")),
        ),
    ];
    for (label, assistant) in cases {
        let mut broken = written(&root, "data")?;
        if let Some(session) = broken.session.as_mut() {
            session.messages[0] = assistant;
        }
        assert!(
            matches!(
                TaskResult::inspect(broken),
                Err(ResultError::InvalidHistory)
            ),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn recorded_read_is_accepted_at_the_limit_and_rejected_above() -> TestResult {
    let root = root()?;
    for (length, accepted) in [(FILE_LIMIT, true), (FILE_LIMIT + 1, false)] {
        let args = json!({});
        let mut read = written(&root, "data")?;
        read.successful_calls = vec![saved("r1", "read_input", &args)];
        read.session = Some(session(
            vec![
                tool_call("r1", "read_input", args.clone(), None),
                tool_result("r1", &"a".repeat(length)),
            ],
            true,
            false,
        ));
        assert_eq!(TaskResult::inspect(read).is_ok(), accepted, "{length}");
    }
    Ok(())
}

#[test]
fn artifact_is_accepted_at_the_limit_and_rejected_above() -> TestResult {
    let root = root()?;
    let output = root.join("output");
    for (length, accepted) in [(FILE_LIMIT, true), (FILE_LIMIT + 1, false)] {
        std::fs::write(&output, "a".repeat(length))?;
        let result = TaskResult::inspect(snapshot(&root, &output, None)?);
        assert_eq!(result.is_ok(), accepted, "{length}");
        if let Ok(result) = result {
            assert_eq!(result.after.len(), length);
        }
    }
    Ok(())
}

#[test]
fn relative_artifact_path_is_rejected_even_when_the_file_exists() -> TestResult {
    let root = root()?;
    assert!(Path::new("Cargo.toml").is_file());
    let snapshot = snapshot(&root, Path::new("Cargo.toml"), None)?;
    assert!(matches!(
        TaskResult::inspect(snapshot),
        Err(ResultError::InvalidArtifact)
    ));
    Ok(())
}
