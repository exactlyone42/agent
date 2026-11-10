use crate::case::Case;
use crate::compile_fixture::compile_fixture;
use crate::fake_server::FakeServer;
use crate::file_task_handler::file_task_handler;
use crate::open_database::open_database;
use crate::outcome::Outcome;
use crate::stored_result::stored_result;
use crate::task_id_of::task_id_of;
use serde_json::{Value, json};
use std::path::PathBuf;

const CANARY: &str = "test-secret-canary";

pub struct FileTaskRun {
    name: &'static str,
    exit_code: i32,
    case: Case,
    source: PathBuf,
    artifact: PathBuf,
    source_content: String,
    output_content: String,
}

impl FileTaskRun {
    pub fn new(name: &'static str, exit_code: i32) -> Self {
        let case = Case::new();
        let source = case.root().join("input.txt");
        let artifact = case.root().join("output.txt");
        let big_source = matches!(name, "read_limit" | "source_grew");
        let source_content = if big_source {
            "x".repeat(1048576)
        } else {
            "original fixture".to_string()
        };
        let output_content = match name {
            "write_limit" => "x".repeat(1048576),
            "write_oversize" => "x".repeat(1048577),
            _ => "ORIGINAL FIXTURE".to_string(),
        };
        std::fs::write(&source, &source_content).unwrap();
        if name == "existing" {
            std::fs::write(&artifact, "keep existing").unwrap();
        }
        if name == "existing_equal" {
            std::fs::write(&artifact, "ORIGINAL FIXTURE").unwrap();
        }
        Self {
            name,
            exit_code,
            case,
            source,
            artifact,
            source_content,
            output_content,
        }
    }

    fn expected(&self) -> Value {
        match self.name {
            "review" | "write_limit" => Value::Null,
            _ => json!("ORIGINAL FIXTURE"),
        }
    }

    pub fn execute(&self) -> (Outcome, Vec<Value>) {
        let server = FakeServer::start(file_task_handler(
            self.name,
            self.source.clone(),
            self.case.database(),
            self.output_content.clone(),
        ));
        let allowed = self.name != "denied";
        let task = self.case.write_task(&json!({
            "endpoint": server.endpoint(),
            "model": "fixture-model",
            "task": {
                "prompt": "讀來源並寫入大寫成果",
                "input": self.source,
                "output": self.artifact,
                "allow_read": allowed,
                "allow_write": allowed && self.name != "write_denied",
                "expected": self.expected()
            }
        }));
        let mut command = self.case.cli(&["run", task.to_str().unwrap()]);
        command.env("AGENT_API_KEY", CANARY);
        if self.name == "write_failed" {
            command
                .env("LD_PRELOAD", compile_fixture(self.case.root()))
                .env("AGENT_TEST_FAIL_WRITE", &self.artifact);
        }
        let outcome = self.case.execute(command);
        (outcome, server.requests())
    }

    fn expected_state(&self) -> &'static str {
        match (self.name, self.exit_code) {
            ("receipt_failed", _) => "running",
            (_, 0) => "succeeded",
            (_, 2) => "needs_review",
            ("redirect" | "corrupted_spec" | "reused_call_id", _) => "outcome_unknown",
            _ => "failed",
        }
    }

    fn check_stored_state(&self, task_id: &str) {
        let (status, events) = stored_result(&self.case, task_id);
        let state = self.expected_state();
        assert_eq!(status["recorded_state"], state);
        let last = if self.name == "receipt_failed" {
            "tool_succeeded"
        } else {
            state
        };
        assert_eq!(events.last().unwrap()["event_type"], last);
    }

    fn check_secret_absent(&self, outcome: &Outcome) {
        for entry in std::fs::read_dir(self.case.state()).unwrap() {
            let bytes = std::fs::read(entry.unwrap().path()).unwrap();
            assert!(
                !bytes
                    .windows(CANARY.len())
                    .any(|part| part == CANARY.as_bytes())
            );
        }
        assert!(!format!("{}{}", outcome.stdout, outcome.stderr).contains(CANARY));
        if self.name == "receipt_failed" {
            open_database(&self.case.database())
                .execute_batch("DROP TRIGGER reject_receipt")
                .unwrap();
        }
    }

    fn check_failure_message(&self, outcome: &Outcome) {
        if self.exit_code != 1 {
            return;
        }
        let message = match self.name {
            "denied" | "write_denied" => "工具未獲批准",
            "receipt_failed" => "任務結果未確認",
            "write_failed" => "成果未完整寫入",
            "redirect" | "corrupted_spec" | "reused_call_id" => "模型執行失敗",
            _ => "沒有可驗證的成果檔案",
        };
        assert!(outcome.stderr.contains(message), "{}", outcome.stderr);
    }

    fn check_artifact(&self, outcome: &Outcome, requests: &[Value]) {
        let read = |path: &PathBuf| std::fs::read_to_string(path).unwrap();
        match self.name {
            "correct" | "review" | "read_limit" | "write_limit" | "source_grew"
            | "duplicate_write" => {
                let report: Value = serde_json::from_str(&outcome.stdout).unwrap();
                let artifact = if self.exit_code == 0 {
                    "verified"
                } else {
                    "awaiting_review"
                };
                assert_eq!(report["artifact"], artifact);
                assert_eq!(read(&self.artifact), self.output_content);
                if self.name == "duplicate_write" {
                    self.check_duplicate_write(requests);
                }
            }
            "wrong" => assert!(!self.artifact.exists()),
            "existing" => assert_eq!(read(&self.artifact), "keep existing"),
            "existing_equal" => assert_eq!(read(&self.artifact), "ORIGINAL FIXTURE"),
            "write_failed" => assert_eq!(read(&self.artifact), ""),
            "receipt_failed" => {
                assert_eq!(outcome.stdout, "");
                assert_eq!(read(&self.artifact), self.output_content);
            }
            _ => assert!(!self.artifact.exists()),
        }
    }

    fn check_duplicate_write(&self, requests: &[Value]) {
        assert_eq!(requests.len(), 4);
        let messages = tool_messages(&requests[3]);
        let last = messages.last().unwrap()["content"].as_str().unwrap();
        assert!(last.contains("成果已寫入，不重複執行"));
    }

    fn check_requests(&self, requests: &[Value]) {
        if matches!(self.name, "redirect" | "corrupted_spec" | "denied") {
            assert_eq!(requests.len(), 1);
            check_definitions(&requests[0]);
            return;
        }
        assert!(requests.len() >= 2);
        if matches!(self.name, "reused_call_id" | "write_denied") {
            assert_eq!(requests.len(), 2);
        }
        let messages = tool_messages(&requests[1]);
        match self.name {
            "source_grew" => {
                let contents: Vec<&str> = messages
                    .iter()
                    .map(|m| m["content"].as_str().unwrap())
                    .collect();
                assert!(
                    contents
                        .iter()
                        .any(|text| text.contains("來源超出大小限制"))
                );
                assert!(!contents.contains(&self.source_content.as_str()));
            }
            _ => assert!(
                messages
                    .iter()
                    .any(|m| m["content"] == self.source_content.as_str())
            ),
        }
        check_definitions(&requests[0]);
    }

    pub fn verify(&self, outcome: &Outcome, requests: &[Value]) {
        assert_eq!(outcome.code(), Some(self.exit_code), "{}", outcome.stderr);
        assert!(!outcome.stderr.contains("逐次確認模式"));
        self.check_stored_state(&task_id_of(&outcome.stderr));
        self.check_secret_absent(outcome);
        self.check_failure_message(outcome);
        let grown = if self.name == "source_grew" { "y" } else { "" };
        let expected_source = format!("{}{grown}", self.source_content);
        assert_eq!(
            std::fs::read_to_string(&self.source).unwrap(),
            expected_source
        );
        self.check_artifact(outcome, requests);
        self.check_requests(requests);
        assert!(!self.case.root().join("unapproved.txt").exists());
    }
}

fn tool_messages(request: &Value) -> Vec<&Value> {
    request["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|message| message["role"] == "tool")
        .collect()
}

fn check_definitions(request: &Value) {
    let find = |name: &str| {
        request["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["function"]["name"] == name)
            .map(|tool| tool["function"].clone())
            .unwrap()
    };
    let read = find("read_input");
    let write = find("write_output");
    assert_eq!(read["description"], "讀取使用者指定且獲准的來源檔案");
    assert_eq!(
        write["description"],
        "建立使用者指定且獲准的新成果檔案，不覆寫既有資料"
    );
    assert_eq!(
        read["parameters"],
        json!({"type": "object", "properties": {}, "additionalProperties": false})
    );
    assert_eq!(
        write["parameters"],
        json!({
            "type": "object",
            "properties": {"content": {"type": "string"}},
            "required": ["content"],
            "additionalProperties": false
        })
    );
}
