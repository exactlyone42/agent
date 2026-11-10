use crate::case::Case;
use agent_platform_agent::{
    store::{ApprovalMode, TaskSpec, TaskStore},
    task::{ArtifactState, FileTask, TaskReport},
};
use serde_json::{Value, json};

#[test]
fn result_reads_saved_source_and_rejects_wrong_actual_output() {
    let case = Case::new();
    std::fs::create_dir(case.state()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(case.state(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let input = case.root().join("source.md");
    let output = case.root().join("result.md");
    std::fs::write(&input, "changed later").unwrap();
    std::fs::write(&output, "fixed").unwrap();
    let spec = TaskSpec {
        workspace: case.root().to_path_buf(),
        endpoint: "http://127.0.0.1:9/v1".into(),
        model: "private-model".into(),
        approval_mode: ApprovalMode::Preapproved,
        task: FileTask {
            prompt: "private-prompt".into(),
            input,
            output: output.clone(),
            expected: Some("fixed".into()),
            allow_read: true,
            allow_write: true,
        },
    };
    let mut store = TaskStore::open(&case.database()).unwrap();
    let (task, generation) = store.start_spec(&spec).unwrap();
    for (step, id, name, args) in [
        (0, "read", "read_input", "{}"),
        (1, "write", "write_output", r#"{"content":"fixed"}"#),
    ] {
        let intent = store
            .prepare_intent(&task, 1, step, id, name, args)
            .unwrap();
        store.claim(&intent).unwrap();
        store.receipt(&task, 1, step, id, true).unwrap();
    }
    let session = json!({"revision":1,"messages":[
        {"User":"private-prompt"},
        {"Assistant":{"text":"","calls":[{"id":"read","name":"read_input","args":{},"bad_args":null}]}},
        {"Tool":{"call_id":"read","content":"original","is_error":false}},
        {"Assistant":{"text":"","calls":[{"id":"write","name":"write_output","args":{"content":"fixed"},"bad_args":null}]}},
        {"Tool":{"call_id":"write","content":"written","is_error":false}}
    ],"next_turn":2,"completed":true,"unresolved_effect":false});
    let connection = rusqlite::Connection::open(case.database()).unwrap();
    connection
        .execute(
            "INSERT INTO sessions VALUES(?1,?2,1,?3)",
            rusqlite::params![task, generation, session.to_string()],
        )
        .unwrap();
    store
        .finish(
            &task,
            generation,
            &Ok(TaskReport {
                artifact: ArtifactState::Verified,
                assistant_message: "private-answer".into(),
            }),
        )
        .unwrap();
    drop(connection);
    drop(store);
    let before = std::fs::read(case.database()).unwrap();
    let run = case.run(&["result", &task]);
    assert_eq!(run.code(), Some(0), "{}", run.stderr);
    let result: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(result["before"], "original");
    assert_eq!(result["after"], "fixed");
    assert_eq!(result["verified"], true);
    assert!(!run.stdout.contains("private-"));
    assert_eq!(std::fs::read(case.database()).unwrap(), before);
    std::fs::write(output, "wrong").unwrap();
    let wrong = case.run(&["result", &task]);
    assert_eq!(wrong.code(), Some(1));
    assert!(wrong.stderr.contains("result_unverified"));
}
