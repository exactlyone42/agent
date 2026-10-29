use crate::case::Case;
use crate::set_mode::set_mode;
use serde_json::json;

#[test]
fn unreadable_database_path_is_not_treated_as_missing_on_create() {
    let case = Case::new();
    let state = case.state();
    std::fs::create_dir(&state).expect("state directory");
    set_mode(&state, 0o700);
    let task = case.write_task(&json!({
        "endpoint": "http://127.0.0.1:1/v1",
        "model": "fixture-model",
        "task": {
            "prompt": "work",
            "input": "unused",
            "output": case.root().join("output"),
            "allow_write": true
        }
    }));
    set_mode(&state, 0o600);
    let result = case.run(&["run", task.to_str().expect("task path")]);
    set_mode(&state, 0o700);
    assert_eq!(result.code(), Some(1));
    assert!(
        result.stderr.contains("找不到或無法讀取任務資料庫"),
        "{}",
        result.stderr
    );
}
