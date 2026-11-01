use crate::case::Case;
use crate::compile_fixture::compile_fixture;
use crate::completion::completion;
use crate::fake_server::FakeServer;
use crate::open_database::open_database;
use crate::outcome::Outcome;
use crate::running::Running;
use crate::stored_result::stored_result;
use crate::sync_pipes::SyncPipes;
use crate::text_message::text_message;
use crate::tool_call_message::tool_call_message;
use crate::write_json::write_json;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::{Duration, Instant};

const SPENT: [&str; 7] = [
    "approve",
    "wrong_revision",
    "busy",
    "receipt_busy",
    "receipt_busy_timeout",
    "receipt_busy_cancel",
    "receipt_failure",
];

pub struct ApprovalScenario {
    action: &'static str,
    case: Case,
    artifact: PathBuf,
    server: FakeServer,
    running: Option<Running>,
    sync: Option<SyncPipes>,
    task_id: String,
    approval: Value,
}

impl ApprovalScenario {
    pub fn start(action: &'static str) -> Self {
        let case = Case::new();
        let artifact = case.root().join("output.txt");
        let server = FakeServer::start(|index, _, stream| {
            let reply = if index == 1 {
                let call = tool_call_message(
                    "approval-write-1",
                    "write_output",
                    &json!({"content": "APPROVED"}),
                );
                completion(call, "tool_calls")
            } else {
                completion(text_message("done"), "stop")
            };
            write_json(stream, &reply);
        });
        let task = case.write_task(&json!({
            "endpoint": server.endpoint(),
            "model": "fixture-model",
            "approval_mode": "manual",
            "task": {
                "prompt": "create approved output",
                "input": "unused",
                "output": artifact,
                "allow_write": true,
                "expected": "APPROVED"
            }
        }));
        let mut command = case.cli(&["run", task.to_str().expect("task path")]);
        let sync = (cfg!(target_os = "linux") && action.starts_with("receipt_busy")).then(|| {
            let library = compile_fixture(case.root());
            SyncPipes::attach(&mut command, &library, &artifact)
        });
        let running = case.spawn(command);
        let first = running
            .stderr_line(Duration::from_secs(5))
            .expect("task identifier missing");
        let second = running
            .stderr_line(Duration::from_secs(5))
            .expect("mode line missing");
        assert!(first.starts_with("任務編號："), "{first}");
        assert!(second.contains("逐次確認模式"), "{second}");
        let task_id = first
            .split('：')
            .nth(1)
            .expect("task id")
            .trim()
            .to_string();
        Self {
            action,
            case,
            artifact,
            server,
            running: Some(running),
            sync,
            task_id,
            approval: Value::Null,
        }
    }

    fn running(&mut self) -> &mut Running {
        self.running.as_mut().expect("process still owned")
    }

    fn approvals(&self) -> Vec<Value> {
        let query = self.case.run(&["approvals", &self.task_id]);
        assert_eq!(query.code(), Some(0), "{}", query.stderr);
        serde_json::from_str(&query.stdout).expect("approvals json")
    }

    fn decide(&self, command: &str, revision: i64) -> Outcome {
        let id = self.approval["approval_id"].as_str().expect("approval id");
        self.case.run(&[command, id, &revision.to_string()])
    }

    fn exec_sql(&self, statement: &str) {
        open_database(&self.case.database())
            .execute_batch(statement)
            .expect("inject sql");
    }

    pub fn wait_pending(&mut self) {
        let until = Instant::now() + Duration::from_secs(5);
        let mut pending = Vec::new();
        while Instant::now() < until && !self.running().exited() {
            pending = self.approvals();
            if !pending.is_empty() {
                break;
            }
            sleep(Duration::from_millis(20));
        }
        assert_eq!(pending.len(), 1);
        self.approval = pending.remove(0);
        assert_eq!(self.approval["status"], "pending");
        assert_eq!(self.approval["scope_path"], self.artifact.to_str().unwrap());
        let args: Value =
            serde_json::from_str(self.approval["args_json"].as_str().unwrap()).unwrap();
        assert_eq!(args, json!({"content": "APPROVED"}));
        assert_eq!(self.approval["uses_remaining"], 1);
        assert!(!self.artifact.exists());
        let (status, _) = stored_result(&self.case, &self.task_id);
        assert_eq!(status["recorded_state"], "waiting_approval");
    }

    fn lock_database_briefly(&mut self) {
        let database = open_database(&self.case.database());
        database.execute_batch("BEGIN IMMEDIATE").unwrap();
        sleep(Duration::from_millis(900));
        assert!(
            !self.running().exited(),
            "temporary database lock stopped the task"
        );
        assert!(!self.artifact.exists());
        database.execute_batch("ROLLBACK").unwrap();
    }

    fn inject_before_decision(&self) {
        if self.action == "claim_failure" {
            self.exec_sql("CREATE TRIGGER reject_dispatch BEFORE INSERT ON events WHEN NEW.event_type='tool_dispatched' BEGIN SELECT RAISE(ABORT,'injected failure'); END");
        }
        if self.action == "receipt_failure" {
            self.exec_sql("CREATE TRIGGER reject_tool_receipt BEFORE INSERT ON events WHEN NEW.event_type='tool_succeeded' BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END");
        }
    }

    fn decide_now(&self) {
        self.inject_before_decision();
        let verb = if self.action == "deny" {
            "deny"
        } else {
            "approve"
        };
        let decision = self.decide(verb, 1);
        assert_eq!(decision.code(), Some(0), "{}", decision.stderr);
    }

    fn hold_receipt_lock(&mut self) {
        let mut sync = self.sync.take().expect("sync pipes");
        let reached = sync.reached(self.running());
        assert!(reached, "artifact sync not reached");
        let database = open_database(&self.case.database());
        database.execute_batch("BEGIN IMMEDIATE").unwrap();
        sync.release();
        match self.action {
            "receipt_busy_timeout" => {
                assert!(self.running().wait_for(Duration::from_secs(5)));
            }
            "receipt_busy_cancel" => {
                sleep(Duration::from_millis(550));
                self.running().signal(libc::SIGINT);
                sleep(Duration::from_millis(100));
            }
            _ => {
                sleep(Duration::from_millis(900));
                assert!(!self.running().exited());
            }
        }
        database.execute_batch("ROLLBACK").unwrap();
    }

    pub fn act(&mut self) {
        if self.action == "wrong_revision" {
            assert_eq!(self.decide("approve", 2).code(), Some(1));
            assert!(!self.artifact.exists());
        }
        if self.action == "busy" {
            self.lock_database_briefly();
        }
        match self.action {
            "expired" => {
                self.exec_sql("UPDATE approvals SET expires_at=0");
                assert_eq!(self.decide("approve", 1).code(), Some(1));
            }
            "cancel" => self.running().signal(libc::SIGINT),
            _ => self.decide_now(),
        }
        if self.action.starts_with("receipt_busy") {
            self.hold_receipt_lock();
        }
    }

    fn check_outcome(&self, outcome: &Outcome) {
        let action = self.action;
        let requests = self.server.requests().len();
        if matches!(
            action,
            "approve" | "wrong_revision" | "busy" | "receipt_busy"
        ) {
            assert_eq!(outcome.code(), Some(0), "{}", outcome.stderr);
            assert_eq!(std::fs::read_to_string(&self.artifact).unwrap(), "APPROVED");
            let report: Value = serde_json::from_str(&outcome.stdout).unwrap();
            assert_eq!(report["artifact"], "verified");
            assert_eq!(requests, 2);
            return;
        }
        let cancelled = matches!(action, "cancel" | "receipt_busy_cancel");
        assert_eq!(
            outcome.code(),
            Some(if cancelled { 130 } else { 1 }),
            "{}",
            outcome.stderr
        );
        if matches!(
            action,
            "receipt_failure" | "receipt_busy_timeout" | "receipt_busy_cancel"
        ) {
            assert_eq!(std::fs::read_to_string(&self.artifact).unwrap(), "APPROVED");
            self.check_receipt_stdout(outcome);
        } else {
            assert!(!self.artifact.exists());
        }
        if matches!(action, "deny" | "expired") {
            assert_eq!(requests, 2);
            let model_requests = self.server.requests();
            let messages = model_requests[1]["messages"].as_array().unwrap();
            assert!(messages.iter().any(|message| {
                message["role"] == "tool"
                    && message["tool_call_id"] == "approval-write-1"
                    && message["content"]
                        .as_str()
                        .is_some_and(|content| content.contains("未獲批准"))
            }));
            return;
        }
        assert_eq!(requests, 1);
    }

    fn check_receipt_stdout(&self, outcome: &Outcome) {
        if self.action != "receipt_busy_cancel" {
            assert_eq!(outcome.stdout, "");
            return;
        }
        let report: Value = serde_json::from_str(&outcome.stdout).unwrap();
        assert_eq!(report["state"], "outcome_unknown");
    }

    fn check_saved_record(&self) {
        assert_eq!(self.decide("approve", 1).code(), Some(1));
        let saved = self.approvals().remove(0);
        let spent = SPENT.contains(&self.action);
        assert_eq!(saved["uses_remaining"], if spent { 0 } else { 1 });
        if matches!(self.action, "receipt_failure" | "receipt_busy_cancel") {
            assert_eq!(saved["intent_status"], "outcome_unknown");
            let (status, events) = stored_result(&self.case, &self.task_id);
            assert_eq!(status["recorded_state"], "outcome_unknown");
            assert_eq!(events.last().unwrap()["event_type"], "tool_outcome_unknown");
        }
        if matches!(self.action, "cancel" | "claim_failure") {
            assert_eq!(saved["status"], "revoked");
        }
    }

    pub fn finish(mut self) {
        let running = self.running.take().expect("process still owned");
        let outcome = running.finish();
        self.check_outcome(&outcome);
        self.check_saved_record();
    }
}
