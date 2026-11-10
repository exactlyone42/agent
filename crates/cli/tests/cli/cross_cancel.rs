use crate::{
    await_flag::await_flag, case::Case, completion::completion, fake_server::FakeServer,
    hold_until_disconnect::hold_until_disconnect, text_message::text_message,
    tool_call_message::tool_call_message, write_json::write_json,
};
use agent_platform_agent::store::TaskStore;
use serde_json::{Value, json};
use std::{
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

#[test]
fn another_cli_cancels_waits_without_writes_and_new_run_requires_fresh_approval() {
    for phase in ["approval", "model"] {
        let case = Case::limited(Duration::from_secs(40));
        let ready = Arc::new(AtomicBool::new(false));
        let disconnected = Arc::new(AtomicBool::new(false));
        let server = FakeServer::start({
            let ready = ready.clone();
            let disconnected = disconnected.clone();
            move |index, _, stream| {
                if phase == "model" && index == 1 {
                    hold_until_disconnect(stream, &ready, &disconnected);
                    return;
                }
                if index <= 2 {
                    let id = if index == 1 { "old-write" } else { "new-write" };
                    let call = tool_call_message(id, "write_output", &json!({"content":"fixed\n"}));
                    write_json(stream, &completion(call, "tool_calls"));
                    return;
                }
                write_json(stream, &completion(text_message("done"), "stop"));
            }
        });
        let task = case.write_task(&json!({
            "endpoint":server.endpoint(),"model":"fixture-model","approval_mode":"manual",
            "task":{"prompt":"create output","input":"unused","output":"output","allow_write":true,"expected":"fixed\n"}
        }));
        let mut original = case.spawn(case.cli(&["run", task.to_str().unwrap()]));
        let line = original.stderr_line(Duration::from_secs(5)).unwrap();
        let task_id = line.strip_prefix("任務編號：").unwrap().trim().to_owned();
        let until = Instant::now() + Duration::from_secs(5);
        while server.requests().is_empty() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(server.requests().len(), 1);
        let mut old_approval = None;
        if phase == "approval" {
            while old_approval.is_none() && Instant::now() < until {
                old_approval = TaskStore::read(&case.database())
                    .unwrap()
                    .approvals(&task_id)
                    .unwrap()
                    .into_iter()
                    .find(|item| item.status == "pending");
                std::thread::sleep(Duration::from_millis(10));
            }
            assert!(old_approval.is_some());
        }
        if phase == "model" {
            assert!(await_flag(&ready, Duration::from_secs(5)));
        }
        let began = Instant::now();
        let cancelled = if phase == "model" {
            let commands = case.root().join("cancel-commands.txt");
            std::fs::write(&commands, "/list\n/cancel 1\n/status\n/exit\n").unwrap();
            let mut command = case.cli(&[]);
            command.stdin(std::fs::File::open(commands).unwrap());
            case.execute(command)
        } else {
            case.run(&["cancel", &task_id])
        };
        assert_eq!(cancelled.code(), Some(0), "{phase}: {}", cancelled.stderr);
        assert!(began.elapsed() < Duration::from_secs(5));
        if phase == "model" {
            assert!(
                cancelled.stdout.contains("任務已取消"),
                "{}",
                cancelled.stdout
            );
            assert!(
                cancelled.stdout.contains("任務狀態：已取消"),
                "{}",
                cancelled.stdout
            );
        } else {
            let acknowledgement: Value = serde_json::from_str(&cancelled.stdout).unwrap();
            assert_eq!(acknowledgement["state"], "cancelled");
            assert_eq!(acknowledgement["confirmed"], true);
            assert_eq!(acknowledgement["confirmation_scope"], "local_task");
            assert_eq!(acknowledgement["external_stop_confirmed"], false);
        }
        assert!(original.wait_for(Duration::from_millis(500)));
        let outcome = original.finish();
        assert_eq!(outcome.code(), Some(130), "{phase}: {}", outcome.stderr);
        let store = TaskStore::read(&case.database()).unwrap();
        let before = store.status(&task_id).unwrap();
        assert_eq!(before.recorded_state, "cancelled");
        let events = store.events(&task_id, 0).unwrap();
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "cancel_requested")
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.event_type == "cancelled")
                .count(),
            1
        );
        let repeated = case.run(&["cancel", &task_id]);
        assert_eq!(repeated.code(), Some(0), "{}", repeated.stderr);
        if let Some(approval) = old_approval {
            assert_eq!(
                case.run(&["approve", &approval.approval_id, "1"]).code(),
                Some(1)
            );
        }
        let observation = Instant::now();
        while observation.elapsed() < Duration::from_secs(10) {
            assert!(!case.root().join("output").exists());
            assert_eq!(server.requests().len(), 1);
            assert_eq!(store.status(&task_id).unwrap().last_seq, before.last_seq);
            std::thread::sleep(Duration::from_millis(50));
        }
        if phase == "model" {
            assert!(await_flag(&disconnected, Duration::from_secs(1)));
        }
        let resumed = case.spawn(case.cli(&["resume", &task_id]));
        let until = Instant::now() + Duration::from_secs(5);
        let mut fresh = None;
        while fresh.is_none() && Instant::now() < until {
            fresh = store
                .approvals(&task_id)
                .unwrap()
                .into_iter()
                .find(|item| item.status == "pending");
            std::thread::sleep(Duration::from_millis(10));
        }
        let fresh = fresh.expect("new run waiting for approval");
        assert_eq!(store.status(&task_id).unwrap().generation, 2);
        assert!(!case.root().join("output").exists());
        assert_eq!(
            case.run(&["approve", &fresh.approval_id, "1"]).code(),
            Some(0)
        );
        let resumed = resumed.finish();
        assert_eq!(resumed.code(), Some(0), "{phase}: {}", resumed.stderr);
        assert_eq!(
            std::fs::read_to_string(case.root().join("output")).unwrap(),
            "fixed\n"
        );
        assert_eq!(store.status(&task_id).unwrap().recorded_state, "succeeded");
    }
}
