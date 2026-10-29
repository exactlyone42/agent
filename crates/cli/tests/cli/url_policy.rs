use crate::case::Case;
use crate::run_each::run_each;
use serde_json::{Value, json};

const REJECTED: [&str; 8] = [
    "http://localhost:1/v1",
    "http://192.0.2.1/v1",
    "http://[2001:db8::1]/v1",
    "ftp://127.0.0.1/v1",
    "http://user@127.0.0.1/v1",
    "http://:secret@127.0.0.1/v1",
    "http://127.0.0.1/v1?key=value",
    "http://127.0.0.1/v1#fragment",
];

fn request(endpoint: &str) -> Value {
    json!({
        "endpoint": endpoint,
        "model": "fixture-model",
        "task": {
            "input": "missing.txt",
            "output": "output.txt",
            "prompt": "work",
            "allow_read": true
        }
    })
}

#[test]
fn url_policy_and_json_size() {
    let case = Case::new();
    let accepted = ["http://127.0.0.1:1/v1", "http://[::1]:1/v1"];
    let endpoints = REJECTED.into_iter().chain(accepted);
    run_each(endpoints, |endpoint| {
        let task = case.write_task(&request(endpoint));
        let result = case.run(&["run", task.to_str().unwrap()]);
        assert_eq!(result.code(), Some(1));
        let message = if REJECTED.contains(&endpoint) {
            "目前只接受"
        } else {
            "無法開啟獲准的來源"
        };
        assert!(result.stderr.contains(message), "{}", result.stderr);
    });
    let payload = request("http://127.0.0.1:1/v1").to_string();
    run_each([65535usize, 65536, 65537], |size| {
        let padded = format!("{payload}{}", " ".repeat(size - payload.len()));
        let task = case.root().join("task.json");
        std::fs::write(&task, padded).unwrap();
        let result = case.run(&["run", task.to_str().unwrap()]);
        assert_eq!(result.code(), Some(1));
        let message = if size > 65536 {
            "任務檔超過"
        } else {
            "無法開啟獲准的來源"
        };
        assert!(result.stderr.contains(message), "{}", result.stderr);
    });
}
