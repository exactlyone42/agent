use crate::case::Case;
use serde_json::json;
use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;

#[test]
fn invalid_key_encoding_fails_before_connection() {
    let case = Case::new();
    let task = case.write_task(&json!({
        "endpoint": "http://127.0.0.1:1/v1",
        "model": "fixture-model",
        "task": {"prompt": "work", "input": "missing.txt", "output": "new.txt"}
    }));
    let mut command = case.cli(&["run", task.to_str().unwrap()]);
    command.env("AGENT_API_KEY", OsString::from_vec(vec![0xff]));
    let result = case.execute(command);
    assert_eq!(result.code(), Some(1));
    assert!(
        result.stderr.contains("AGENT_API_KEY 格式錯誤"),
        "{}",
        result.stderr
    );
    assert!(!case.root().join("new.txt").exists());
}
