use crate::{
    await_flag::await_flag, case::Case, completion::completion, fake_server::FakeServer,
    hold_until_disconnect::hold_until_disconnect, stored_result::stored_result,
    text_message::text_message, tool_call_message::tool_call_message, write_json::write_json,
};
use serde_json::json;
use std::{
    process::Stdio,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

#[test]
fn slash_cancel_stops_approval_and_model_waits_and_saves_cancelled_state() {
    for (phase, action) in ["approval", "model", "written"]
        .into_iter()
        .flat_map(|phase| ["cancel", "eof", "parent"].map(move |action| (phase, action)))
    {
        let case = Case::limited(Duration::from_secs(15));
        let artifact = case.root().join("output.txt");
        let source = case.root().join("input.txt");
        std::fs::write(&source, "preserve source").unwrap();
        let ready = Arc::new(AtomicBool::new(false));
        let disconnected = Arc::new(AtomicBool::new(false));
        let server = FakeServer::start({
            let ready = ready.clone();
            let disconnected = disconnected.clone();
            move |index, _, stream| {
                if phase == "model" || (phase == "written" && index > 1) {
                    hold_until_disconnect(stream, &ready, &disconnected);
                    return;
                }
                let message = if index == 1 {
                    tool_call_message(
                        "pending-write",
                        "write_output",
                        &json!({"content":"fixture output"}),
                    )
                } else {
                    text_message("done")
                };
                write_json(
                    stream,
                    &completion(message, if index == 1 { "tool_calls" } else { "stop" }),
                );
            }
        });
        let task = case.write_task(&json!({"endpoint":server.endpoint(),"model":"fixture-model","approval_mode":if phase == "approval" { "manual" } else { "preapproved" },"task":{"prompt":"work","input":source,"output":artifact,"expected":"fixture output","allow_write":true}}));
        let mut command = case.cli(&[]);
        command.stdin(Stdio::piped());
        let mut running = case.spawn(command);
        if action == "cancel" {
            running.input("/cancel extra");
            running.input("/cancel");
        }
        running.input(&format!("/run {}", task.display()));
        if phase == "approval" {
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut shown = false;
            while Instant::now() < deadline && !shown {
                shown = running
                    .stdout_line(Duration::from_millis(100))
                    .is_some_and(|line| line.starts_with("輸入 /approve 或 /deny"));
            }
            assert!(shown, "approval was not displayed");
        } else {
            assert!(await_flag(&ready, Duration::from_secs(5)));
        }
        let connection = rusqlite::Connection::open(case.database()).unwrap();
        let task_id: String = connection
            .query_row("SELECT task_id FROM tasks", [], |r| r.get(0))
            .unwrap();
        drop(connection);
        match action {
            "cancel" => {
                running.input("/cancel");
                running.input("/approvals");
                running.input("/approve");
            }
            "eof" => running.close_input(),
            "parent" => running.stop_parent(),
            _ => unreachable!(),
        }
        let deadline = Instant::now() + Duration::from_secs(3);
        let mut state = String::new();
        while Instant::now() < deadline && state != "cancelled" {
            state = agent_platform_agent::store::TaskStore::read(&case.database())
                .unwrap()
                .status(&task_id)
                .unwrap()
                .recorded_state;
            std::thread::sleep(Duration::from_millis(10));
        }
        if state != "cancelled" && phase == "approval" {
            let pending = case.run(&["approvals", &task_id]);
            let records: serde_json::Value = serde_json::from_str(&pending.stdout).unwrap();
            let item = records
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["status"] == "pending")
                .unwrap();
            let revision = item["spec_revision"].as_i64().unwrap().to_string();
            let denied = case.run(&["deny", item["approval_id"].as_str().unwrap(), &revision]);
            assert_eq!(denied.code(), Some(0));
        }
        if action == "cancel" {
            running.input("/status");
            let deadline = Instant::now() + Duration::from_secs(3);
            let mut displayed = false;
            while Instant::now() < deadline && !displayed {
                displayed = running
                    .stdout_line(Duration::from_millis(100))
                    .is_some_and(|line| line.contains("任務狀態：已取消"));
            }
            assert!(displayed);
        }
        running.close_input();
        let outcome = running.finish();
        assert_eq!(state, "cancelled", "{phase}/{action}: {}", outcome.stdout);
        if action != "parent" {
            assert_eq!(outcome.code(), Some(0), "{}", outcome.stderr);
        }
        if action == "cancel" {
            assert!(outcome.stdout.contains("任務已取消"), "{}", outcome.stdout);
            assert!(
                outcome.stdout.contains("請使用 /list 顯示的正整數序號"),
                "{}",
                outcome.stdout
            );
            assert!(
                outcome.stdout.contains("尚無執行中的任務"),
                "{}",
                outcome.stdout
            );
            assert!(!outcome.stdout.contains("已批准"), "{}", outcome.stdout);
        }
        let (status, events) = stored_result(&case, &task_id);
        assert_eq!(status["recorded_state"], "cancelled");
        assert_eq!(status["artifact_created"], phase == "written");
        assert!(
            events
                .iter()
                .any(|event| event["event_type"] == "cancelled")
        );
        if phase == "approval" {
            assert!(
                events
                    .iter()
                    .any(|event| event["event_type"] == "approval_revoked")
            );
        }
        let dispatched = events
            .iter()
            .filter(|event| event["event_type"] == "tool_dispatched")
            .count();
        assert_eq!(dispatched, usize::from(phase == "written"));
        assert_eq!(
            server.requests().len(),
            if phase == "written" { 2 } else { 1 }
        );
        if phase == "written" {
            assert_eq!(
                std::fs::read_to_string(&artifact).unwrap(),
                "fixture output"
            );
        } else {
            assert!(!artifact.exists());
        }
        assert_eq!(std::fs::read_to_string(source).unwrap(), "preserve source");
        if phase != "approval" {
            assert!(await_flag(&disconnected, Duration::from_secs(2)));
        }
    }
}
