use crate::case::Case;
use crate::make_fifo::make_fifo;
use serde_json::json;
use std::time::Duration;

#[test]
fn fifo_source_is_rejected_without_waiting_for_a_writer() {
    let case = Case::limited(Duration::from_secs(10));
    let source = case.root().join("source");
    make_fifo(&source);
    let output = case.root().join("output");
    let task = case.write_task(&json!({
        "endpoint": "http://127.0.0.1:1/v1",
        "model": "fixture-model",
        "task": {"input": source, "output": output, "prompt": "work", "allow_read": true}
    }));
    let result = case.run(&["run", task.to_str().unwrap()]);
    assert_eq!(result.code(), Some(1));
    assert!(result.stderr.contains("來源必須是"), "{}", result.stderr);
    assert!(!output.exists());
    let as_config = case.run(&["run", source.to_str().unwrap()]);
    assert_eq!(as_config.code(), Some(1));
    assert!(
        as_config.stderr.contains("無法開啟任務檔"),
        "{}",
        as_config.stderr
    );
}
