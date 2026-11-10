use crate::case::Case;
use agent_platform_agent::{
    store::{ApprovalMode, TaskSpec, TaskStore},
    task::{FileTask, TaskError},
};
use serde_json::Value;

fn create_state(case: &Case) {
    let mut directory = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        directory.mode(0o700);
    }
    directory.create(case.state()).unwrap();
}

fn store(case: &Case) -> TaskStore {
    create_state(case);
    TaskStore::open(&case.database()).unwrap()
}

fn spec(case: &Case) -> TaskSpec {
    TaskSpec {
        workspace: case.root().join("工作區"),
        endpoint: "http://127.0.0.1:9".into(),
        model: "private-model-canary".into(),
        approval_mode: ApprovalMode::Manual,
        task: FileTask {
            prompt: "private-prompt-canary".into(),
            input: "input.txt".into(),
            output: "output.txt".into(),
            expected: Some("private-expected-canary".into()),
            allow_read: true,
            allow_write: true,
        },
    }
}

#[test]
fn list_tasks_pages_without_duplicates_or_private_content() {
    let case = Case::new();
    let mut store = store(&case);
    let spec = spec(&case);
    let mut ids = Vec::new();
    for _ in 0..103 {
        ids.push(store.start_spec(&spec).unwrap().0);
    }
    store
        .finish(&ids[0], 1, &Err(TaskError::MissingArtifact))
        .unwrap();
    ids.sort();
    drop(store);
    let before = std::fs::read(case.database()).unwrap();
    let first = case.run(&["list"]);
    assert_eq!(first.code(), Some(0), "{}", first.stderr);
    let page: Value = serde_json::from_str(&first.stdout).unwrap();
    assert_eq!(page["schema_version"], 1);
    assert_eq!(page["tasks"].as_array().unwrap().len(), 100);
    assert_eq!(page["next_cursor"], ids[99]);
    let second = case.run(&["list", page["next_cursor"].as_str().unwrap()]);
    assert_eq!(second.code(), Some(0), "{}", second.stderr);
    let next: Value = serde_json::from_str(&second.stdout).unwrap();
    assert!(next["next_cursor"].is_null());
    let items: Vec<_> = page["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .chain(next["tasks"].as_array().unwrap())
        .collect();
    assert_eq!(
        items
            .iter()
            .map(|item| item["task_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ids
    );
    assert!(
        items
            .iter()
            .all(|item| item["workspace"].as_str() == spec.workspace.to_str())
    );
    assert_eq!(
        items
            .iter()
            .filter(|item| item["recorded_state"] == "failed")
            .count(),
        1
    );
    assert_eq!(
        items
            .iter()
            .filter(|item| item["recorded_state"] == "running")
            .count(),
        102
    );
    assert!(!format!("{}{}", first.stdout, second.stdout).contains("canary"));
    assert_eq!(std::fs::read(case.database()).unwrap(), before);
}

#[test]
fn list_tasks_empty_legacy_and_invalid_data_are_explicit() {
    let case = Case::new();
    let mut store = store(&case);
    let empty = case.run(&["list"]);
    assert_eq!(empty.code(), Some(0), "{}", empty.stderr);
    let page: Value = serde_json::from_str(&empty.stdout).unwrap();
    assert_eq!(page["tasks"], serde_json::json!([]));
    assert!(page["next_cursor"].is_null());
    let legacy = store.start().unwrap().0;
    let result = case.run(&["list"]);
    let page: Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(page["tasks"][0]["task_id"], legacy);
    assert!(page["tasks"][0]["workspace"].is_null());
    let task = store.start_spec(&spec(&case)).unwrap().0;
    let connection = crate::open_database::open_database(&case.database());
    connection
        .execute(
            "UPDATE task_specs SET spec_json='{}' WHERE task_id=?1",
            [&task],
        )
        .unwrap();
    let bad = case.run(&["list"]);
    assert_eq!(bad.code(), Some(1));
    assert!(bad.stdout.is_empty());
    assert!(bad.stderr.contains("invalid_task_spec"));
    let bad_cursor = case.run(&["list", "not-a-cursor"]);
    assert_eq!(bad_cursor.code(), Some(1));
    assert!(bad_cursor.stdout.is_empty());
    let extra = case.run(&["list", &task, "extra"]);
    assert_eq!(extra.code(), Some(1));
    let missing = Case::new();
    assert_eq!(missing.run(&["list"]).code(), Some(1));
    assert!(!missing.state().exists());
}

#[test]
fn slash_list_advances_without_typing_task_ids() {
    let case = Case::new();
    let mut store = store(&case);
    let mut ids = Vec::new();
    for _ in 0..103 {
        ids.push(store.start_spec(&spec(&case)).unwrap().0);
    }
    let labels = [
        ("running", "執行中（最後保存狀態）"),
        ("waiting_approval", "等待批准"),
        ("succeeded", "成果已驗證"),
        ("needs_review", "成果待人工核對"),
        ("failed", "失敗"),
        ("cancelled", "已取消"),
        ("outcome_unknown", "結果未確認，請核對成果"),
    ];
    let connection = crate::open_database::open_database(&case.database());
    for (index, (state, _)) in labels.iter().enumerate() {
        connection
            .execute(
                "UPDATE tasks SET recorded_state=?1 WHERE task_id=?2",
                [state, &ids[index].as_str()],
            )
            .unwrap();
    }
    drop(connection);
    ids.sort();
    drop(store);
    let input = case.root().join("commands.txt");
    std::fs::write(
        &input,
        b"/list next\n/list\n/list next\n/list next\n/list wrong\n/exit\n",
    )
    .unwrap();
    let mut command = case.cli(&[]);
    command.stdin(std::fs::File::open(input).unwrap());
    let result = case.execute(command);
    assert_eq!(result.code(), Some(0), "{}", result.stderr);
    for id in ids {
        assert_eq!(result.stdout.matches(&id).count(), 1, "{}", result.stdout);
    }
    for (_, label) in labels {
        assert!(result.stdout.contains(label), "{}", result.stdout);
    }
    assert!(result.stdout.contains("沒有下一頁"));
    assert!(result.stdout.contains("/list next"));
    assert!(!result.stdout.contains("canary"));
}

#[test]
fn list_tasks_reads_v1_without_upgrading_and_rejects_broken_database() {
    let case = Case::new();
    create_state(&case);
    let connection = crate::open_database::open_database(&case.database());
    connection
        .execute_batch(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../agent/src/store_v1.sql"
        )))
        .unwrap();
    connection
        .pragma_update(None, "application_id", 0x41504754)
        .unwrap();
    connection.pragma_update(None, "user_version", 1).unwrap();
    let id = "00000000-0000-0000-0000-000000000001";
    connection
        .execute(
            "INSERT INTO tasks(task_id,recorded_state,last_seq) VALUES (?1,'running',1)",
            [id],
        )
        .unwrap();
    drop(connection);
    let before = std::fs::read(case.database()).unwrap();
    let result = case.run(&["list"]);
    assert_eq!(result.code(), Some(0), "{}", result.stderr);
    let page: Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(page["tasks"].as_array().unwrap().len(), 1);
    assert_eq!(page["tasks"][0]["task_id"], id);
    assert!(page["tasks"][0]["workspace"].is_null());
    assert_eq!(std::fs::read(case.database()).unwrap(), before);
    let connection = crate::open_database::open_database(&case.database());
    let version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 1);
    drop(connection);
    std::fs::write(case.database(), b"broken database").unwrap();
    let broken = case.run(&["list"]);
    assert_eq!(broken.code(), Some(1));
    assert!(broken.stdout.is_empty());
    assert_eq!(std::fs::read(case.database()).unwrap(), b"broken database");
}
