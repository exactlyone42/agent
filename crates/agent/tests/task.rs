use agent_platform_agent::task::{
    CancellationToken, FileTask, TASK_TIMEOUT, TaskError, run_file_task,
};
use rig_core::test_utils::{MockCompletionModel, MockTurn};
use std::path::PathBuf;

fn task(prompt: String, expected: Option<String>) -> FileTask {
    FileTask {
        prompt,
        expected,
        input: PathBuf::from("not-authorized-input"),
        output: PathBuf::from("not-authorized-output"),
        allow_read: false,
        allow_write: false,
    }
}

#[tokio::test]
async fn cancelled_task_never_opens_input_or_requests_model() {
    let model = MockCompletionModel::from_turns([MockTurn::text("done")]);
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let mut request = task("work".into(), None);
    request.allow_read = true;
    let result = run_file_task(model.clone(), request, cancellation, (), TASK_TIMEOUT).await;
    assert!(matches!(
        result,
        Err(TaskError::Cancelled {
            artifact_created: false
        })
    ));
    assert!(model.requests().is_empty());
}

#[tokio::test]
async fn prompt_and_expected_limits_are_checked_before_model_requests() {
    for (prompt, expected, valid) in [
        (String::new(), None, false),
        (" \n\t".into(), None, false),
        ("x".repeat(1_048_575), None, true),
        ("x".repeat(1_048_576), None, true),
        ("x".repeat(1_048_577), None, false),
        ("work".into(), Some("x".repeat(1_048_575)), true),
        ("work".into(), Some("x".repeat(1_048_576)), true),
        ("work".into(), Some("x".repeat(1_048_577)), false),
    ] {
        let model = MockCompletionModel::from_turns([MockTurn::text("done")]);
        let result = run_file_task(
            model.clone(),
            task(prompt, expected),
            CancellationToken::new(),
            (),
            TASK_TIMEOUT,
        )
        .await;
        if valid {
            assert!(
                matches!(result, Err(TaskError::MissingArtifact)),
                "{result:?}"
            );
            assert_eq!(model.requests().len(), 1);
        } else {
            assert!(matches!(result, Err(TaskError::InvalidTask)), "{result:?}");
            assert!(model.requests().is_empty());
        }
    }
}

#[tokio::test]
async fn input_type_and_size_are_checked_before_model_requests()
-> Result<(), Box<dyn std::error::Error>> {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let root = std::env::temp_dir().join(format!("agent-input-{}-{stamp}", std::process::id()));
    std::fs::create_dir(&root)?;
    let path = root.join("input");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    drop(file);
    for size in [0, 1_048_575, 1_048_576, 1_048_577] {
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)?
            .set_len(size)?;
        let model = MockCompletionModel::from_turns([MockTurn::text("done")]);
        let mut request = task("work".into(), None);
        request.input = path.clone();
        request.allow_read = true;
        let result = run_file_task(
            model.clone(),
            request,
            CancellationToken::new(),
            (),
            TASK_TIMEOUT,
        )
        .await;
        if size <= 1_048_576 {
            assert!(
                matches!(result, Err(TaskError::MissingArtifact)),
                "{result:?}"
            );
            assert_eq!(model.requests().len(), 1);
        } else {
            assert!(matches!(result, Err(TaskError::InvalidInput)), "{result:?}");
            assert!(model.requests().is_empty());
        }
    }
    for (input, exists) in [(root.clone(), true), (root.join("missing"), false)] {
        let model = MockCompletionModel::from_turns([MockTurn::text("done")]);
        let mut request = task("work".into(), None);
        request.input = input;
        request.allow_read = true;
        let result = run_file_task(
            model.clone(),
            request,
            CancellationToken::new(),
            (),
            TASK_TIMEOUT,
        )
        .await;
        if exists {
            assert!(matches!(result, Err(TaskError::InvalidInput)), "{result:?}");
        } else {
            assert!(matches!(result, Err(TaskError::Input(_))), "{result:?}");
        }
        assert!(model.requests().is_empty());
    }
    std::fs::remove_file(path)?;
    std::fs::remove_dir(root)?;
    Ok(())
}
