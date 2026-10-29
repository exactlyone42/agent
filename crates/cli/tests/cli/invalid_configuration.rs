use crate::case::Case;
use crate::run_each::run_each;
use serde_json::json;

#[test]
fn invalid_configuration_does_not_start_task() {
    let case = Case::new();
    let task = case.root().join("task.json");
    let oversized = "x".repeat(65537);
    let remote = json!({
        "endpoint": "https://example.com/v1",
        "model": "fixture-model",
        "task": {
            "input": "missing.txt",
            "output": "new.txt",
            "prompt": "do work",
            "allow_read": true,
            "allow_write": true
        }
    })
    .to_string();
    let values = ["{", oversized.as_str(), remote.as_str()];
    run_each(values, |value| {
        std::fs::write(&task, value).unwrap();
        let result = case.run(&["run", task.to_str().unwrap()]);
        assert_eq!(result.code(), Some(1));
        assert!(!case.root().join("new.txt").exists());
        assert!(!result.stderr.contains("無法開啟獲准的來源"));
    });
}
