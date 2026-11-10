use crate::{
    case::Case, completion::completion, fake_server::FakeServer,
    hold_until_disconnect::hold_until_disconnect, running::Running, text_message::text_message,
    tool_call_message::tool_call_message, write_json::write_json,
};
use agent_platform_agent::store::{ApprovalRecord, TaskStore};
use serde_json::json;
use std::{
    process::Stdio,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

struct ResumeScenario {
    case: Case,
    server: FakeServer,
    ready: Arc<AtomicBool>,
    disconnected: Arc<AtomicBool>,
}

impl ResumeScenario {
    fn new(stage: &'static str) -> Self {
        let case = Case::new();
        std::fs::write(case.root().join("input"), "source\n").unwrap();
        let ready = Arc::new(AtomicBool::new(false));
        let disconnected = Arc::new(AtomicBool::new(false));
        let server = FakeServer::start({
            let ready = ready.clone();
            let disconnected = disconnected.clone();
            move |index, _, stream| {
                if stage == "written" && index == 2 {
                    hold_until_disconnect(stream, &ready, &disconnected);
                    return;
                }
                let reply = if index == 1
                    || (stage == "pending" && index == 2)
                    || (stage == "written" && index == 3)
                {
                    let id = if index == 1 { "old-write" } else { "new-write" };
                    let mut message =
                        tool_call_message(id, "write_output", &json!({"content":"fixed\n"}));
                    message["reasoning"] = json!("preserve fixture reasoning");
                    if stage == "written" && index == 1 {
                        let read = tool_call_message("old-read", "read_input", &json!({}));
                        message["tool_calls"]
                            .as_array_mut()
                            .unwrap()
                            .insert(0, read["tool_calls"][0].clone());
                    }
                    completion(message, "tool_calls")
                } else {
                    completion(text_message("done"), "stop")
                };
                write_json(stream, &reply);
            }
        });
        Self {
            case,
            server,
            ready,
            disconnected,
        }
    }

    fn start(&self, manual: bool) -> (Running, String) {
        let task = self.case.write_task(&json!({
            "endpoint":self.server.endpoint(),"model":"fixture-model",
            "approval_mode":if manual {"manual"} else {"preapproved"},
            "task":{"prompt":"repair source","input":"input","output":"output","allow_read":true,"allow_write":true,"expected":"fixed\n"}
        }));
        let running = self
            .case
            .spawn(self.case.cli(&["run", task.to_str().unwrap()]));
        let line = running.stderr_line(Duration::from_secs(5)).unwrap();
        let task = line.strip_prefix("任務編號：").unwrap().trim().to_owned();
        if manual {
            assert!(
                running
                    .stderr_line(Duration::from_secs(5))
                    .unwrap()
                    .contains("逐次確認模式")
            );
        }
        (running, task)
    }

    fn pending(&self, task: &str, generation: i64) -> ApprovalRecord {
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            let store = TaskStore::read(&self.case.database()).unwrap();
            if store.status(task).unwrap().generation == generation {
                let pending = store
                    .approvals(task)
                    .unwrap()
                    .into_iter()
                    .find(|item| item.status == "pending");
                if let Some(item) = pending {
                    return item;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("missing pending approval for generation {generation}");
    }

    fn session(&self) -> String {
        let connection = crate::open_database::open_database(&self.case.database());
        connection
            .query_row("SELECT session_json FROM sessions", [], |row| row.get(0))
            .unwrap()
    }

    fn stdout_until(running: &Running, needle: &str) -> String {
        let until = Instant::now() + Duration::from_secs(5);
        let mut text = String::new();
        while Instant::now() < until {
            if let Some(line) = running.stdout_line(Duration::from_millis(50)) {
                text.push_str(&line);
                text.push('\n');
                if text.contains(needle) {
                    return text;
                }
            }
        }
        panic!("missing {needle}: {text}");
    }

    fn assert_archived(&self, task: &str, before: &str) {
        let connection = crate::open_database::open_database(&self.case.database());
        let archived: String = connection
            .query_row(
                "SELECT session_json FROM task_runs WHERE task_id=?1 AND generation=1",
                [task],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(archived, before);
        let store = TaskStore::read(&self.case.database()).unwrap();
        assert_eq!(store.status(task).unwrap().generation, 2);
        let events = store.events(task, 0).unwrap();
        let boundary = events
            .iter()
            .position(|event| event.generation == 2 && event.event_type == "started")
            .unwrap();
        assert!(events[..boundary].iter().all(|event| event.generation == 1));
        assert!(events[boundary..].iter().all(|event| event.generation == 2));
    }
}

#[test]
fn killed_approval_wait_resumes_from_numbered_list_with_fresh_approval() {
    let scenario = ResumeScenario::new("pending");
    let (mut original, task) = scenario.start(true);
    let old = scenario.pending(&task, 1);
    let before = scenario.session();
    let conflict = scenario.case.run(&["resume", &task]);
    assert_eq!(conflict.code(), Some(1), "{}", conflict.stderr);
    assert!(conflict.stderr.contains("session_unavailable"));
    assert_eq!(scenario.server.requests().len(), 1);
    original.stop_parent();
    assert!(!original.finish().status.success());
    let mut command = scenario.case.cli(&[]);
    command.stdin(Stdio::piped());
    let mut resumed = scenario.case.spawn(command);
    ResumeScenario::stdout_until(&resumed, "互動模式");
    resumed.input("/list");
    ResumeScenario::stdout_until(&resumed, "1. 任務：");
    resumed.input("/resume 1");
    ResumeScenario::stdout_until(&resumed, "輸入 /approve 或 /deny\n");
    let fresh = scenario.pending(&task, 2);
    assert_ne!(fresh.approval_id, old.approval_id);
    assert_eq!(
        scenario
            .case
            .run(&["approve", &old.approval_id, "1"])
            .code(),
        Some(1)
    );
    assert!(!scenario.case.root().join("output").exists());
    let request = &scenario.server.requests()[1];
    let progress = request["messages"].as_array().unwrap().last().unwrap()["content"]
        .as_str()
        .unwrap();
    assert!(progress.contains("read_input：成功 0 次，未執行 0 次"));
    assert!(progress.contains("write_output：成功 0 次，未執行 1 次"));
    assert!(request["messages"].as_array().unwrap().iter().any(|message|message["tool_call_id"]=="old-write" && message["content"]=="未執行"));
    assert!(
        request["messages"]
            .as_array()
            .unwrap()
            .iter()
            .any(|message| message["reasoning"] == "preserve fixture reasoning")
    );
    resumed.input("/approve");
    ResumeScenario::stdout_until(&resumed, "成果已驗證");
    resumed.input("/result");
    ResumeScenario::stdout_until(&resumed, "成果重新驗證通過");
    resumed.input("/exit");
    resumed.close_input();
    let outcome = resumed.finish();
    assert_eq!(outcome.code(), Some(0), "{}", outcome.stderr);
    assert_eq!(
        std::fs::read_to_string(scenario.case.root().join("output")).unwrap(),
        "fixed\n"
    );
    scenario.assert_archived(&task, &before);
    let store = TaskStore::read(&scenario.case.database()).unwrap();
    let approvals = store.approvals(&task).unwrap();
    assert_eq!(approvals[0].status, "revoked");
    assert_eq!(approvals[0].intent_status, "cancelled");
    assert_eq!(approvals[1].intent_status, "succeeded");
}

#[test]
fn killed_after_write_restores_receipted_file_without_rewriting_and_reads_old_source() {
    let scenario = ResumeScenario::new("written");
    let (mut original, task) = scenario.start(false);
    assert!(original.wait_flag(&scenario.ready, Duration::from_secs(5)));
    let before = scenario.session();
    let output = scenario.case.root().join("output");
    let modified = std::fs::metadata(&output).unwrap().modified().unwrap();
    original.stop_parent();
    assert!(!original.finish().status.success());
    let resumed = scenario.case.spawn(scenario.case.cli(&["resume", &task]));
    assert!(
        resumed
            .stderr_line(Duration::from_secs(5))
            .unwrap()
            .contains(&task)
    );
    assert!(
        resumed
            .stderr_line(Duration::from_secs(5))
            .unwrap()
            .contains("逐次確認模式")
    );
    let pending = scenario.pending(&task, 2);
    assert_eq!(pending.status, "pending");
    assert_eq!(
        std::fs::metadata(&output).unwrap().modified().unwrap(),
        modified
    );
    assert_eq!(
        scenario
            .case
            .run(&["deny", &pending.approval_id, "1"])
            .code(),
        Some(0)
    );
    let resumed = resumed.finish();
    assert_eq!(resumed.code(), Some(0), "{}", resumed.stderr);
    assert_eq!(
        std::fs::metadata(&output).unwrap().modified().unwrap(),
        modified
    );
    let readback = scenario.case.run(&["result", &task]);
    assert_eq!(readback.code(), Some(0), "{}", readback.stderr);
    let result: serde_json::Value = serde_json::from_str(&readback.stdout).unwrap();
    assert_eq!(result["verified"], true);
    assert_eq!(result["before"], "source\n");
    assert_eq!(result["after"], "fixed\n");
    assert_eq!(
        std::fs::read_to_string(scenario.case.root().join("input")).unwrap(),
        "source\n"
    );
    scenario.assert_archived(&task, &before);
    assert_eq!(
        TaskStore::read(&scenario.case.database())
            .unwrap()
            .approvals(&task)
            .unwrap()
            .len(),
        3
    );
    assert_eq!(scenario.server.requests().len(), 4);
    assert!(
        scenario
            .disconnected
            .load(std::sync::atomic::Ordering::SeqCst)
    );
}

#[test]
fn uncertain_write_refuses_resume_without_model_request_or_record_changes() {
    let scenario = ResumeScenario::new("unknown");
    let (original, task) = scenario.start(true);
    let pending = scenario.pending(&task, 1);
    let connection = crate::open_database::open_database(&scenario.case.database());
    connection.execute_batch("CREATE TRIGGER reject_resume_receipt BEFORE INSERT ON events WHEN NEW.event_type='tool_succeeded' BEGIN SELECT RAISE(ABORT,'injected receipt failure'); END;").unwrap();
    assert_eq!(
        scenario
            .case
            .run(&["approve", &pending.approval_id, "1"])
            .code(),
        Some(0)
    );
    assert_eq!(original.finish().code(), Some(1));
    connection
        .execute_batch("DROP TRIGGER reject_resume_receipt")
        .unwrap();
    let store = TaskStore::read(&scenario.case.database()).unwrap();
    let before = store.status(&task).unwrap();
    assert_eq!(before.recorded_state, "outcome_unknown");
    let session = scenario.session();
    let resumed = scenario.case.run(&["resume", &task]);
    assert_eq!(resumed.code(), Some(1), "{}", resumed.stderr);
    assert!(resumed.stderr.contains("resume_unsettled"));
    let after = store.status(&task).unwrap();
    assert_eq!(
        (after.generation, after.last_seq),
        (before.generation, before.last_seq)
    );
    assert_eq!(scenario.session(), session);
    let runs: i64 = connection
        .query_row("SELECT count(*) FROM task_runs", [], |row| row.get(0))
        .unwrap();
    assert_eq!(runs, 0);
    assert_eq!(scenario.server.requests().len(), 1);
    assert_eq!(
        std::fs::read_to_string(scenario.case.root().join("output")).unwrap(),
        "fixed\n"
    );
}
