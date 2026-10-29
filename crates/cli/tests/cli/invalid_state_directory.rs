use crate::case::Case;
use crate::run_each::run_each;
use serde_json::json;

#[test]
fn invalid_state_directory_prevents_start() {
    let case = Case::new();
    let output = case.root().join("output");
    let task = case.write_task(&json!({
        "endpoint": "http://127.0.0.1:1/v1",
        "model": "fixture-model",
        "task": {"prompt": "work", "input": "unused", "output": output, "allow_write": true}
    }));
    let task_path = task.to_str().expect("task path").to_string();
    let cases = [
        ("", "不可為空"),
        ("relative/path", "必須是絕對路徑"),
        (task_path.as_str(), "無法建立任務資料目錄"),
    ];
    run_each(cases, |(value, message)| {
        let mut command = case.cli(&["run", &task_path]);
        command.env("AGENT_STATE_DIR", value);
        let result = case.execute(command);
        assert_eq!(result.code(), Some(1));
        assert!(result.stderr.contains(message), "{}", result.stderr);
        assert!(!result.stderr.contains("任務編號："));
        assert!(!output.exists());
    });
}
