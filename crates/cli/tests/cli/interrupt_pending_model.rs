use crate::await_flag::await_flag;
use crate::case::Case;
use crate::compile_fixture::compile_fixture;
use crate::completion::completion;
use crate::fake_server::FakeServer;
use crate::hold_until_disconnect::hold_until_disconnect;
use crate::run_each::run_each;
use crate::stored_result::stored_result;
use crate::task_id_of::task_id_of;
use crate::tool_call_message::tool_call_message;
use crate::write_json::write_json;
use serde_json::{Value, json};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

const SIGNALS: [i32; 4] = [libc::SIGINT, libc::SIGTERM, libc::SIGHUP, libc::SIGKILL];

fn interrupt_case((already_written, write_failed, signal): (bool, bool, i32)) {
    if write_failed && !cfg!(target_os = "linux") {
        return;
    }
    let case = Case::new();
    let artifact = case.root().join("output.txt");
    let ready = Arc::new(AtomicBool::new(false));
    let disconnected = Arc::new(AtomicBool::new(false));
    let server = FakeServer::start({
        let ready = ready.clone();
        let disconnected = disconnected.clone();
        move |index, _, stream| {
            if already_written && index == 1 {
                let call = tool_call_message(
                    "write-once",
                    "write_output",
                    &json!({"content": "preserve this"}),
                );
                write_json(stream, &completion(call, "tool_calls"));
                return;
            }
            hold_until_disconnect(stream, &ready, &disconnected);
        }
    });
    let task = case.write_task(&json!({
        "endpoint": server.endpoint(),
        "model": "fixture-model",
        "task": {
            "prompt": "produce output",
            "input": "unused.txt",
            "output": artifact,
            "allow_write": true,
            "expected": "preserve this"
        }
    }));
    let mut command = case.cli(&["run", task.to_str().unwrap()]);
    if write_failed {
        command
            .env("LD_PRELOAD", compile_fixture(case.root()))
            .env("AGENT_TEST_FAIL_WRITE", &artifact);
    }
    let mut running = case.spawn(command);
    assert!(
        running.wait_flag(&ready, Duration::from_secs(10)),
        "model request did not reach cancellation boundary"
    );
    running.signal(signal);
    let outcome = running.finish();
    if signal == libc::SIGKILL {
        assert_eq!(outcome.signal(), Some(libc::SIGKILL));
        assert_eq!(outcome.stdout, "");
        let (status, events) = stored_result(&case, &task_id_of(&outcome.stderr));
        assert_eq!(status["recorded_state"], "running");
        assert_eq!(events.len(), if already_written { 4 } else { 1 });
        assert_eq!(status["artifact_created"], Value::Null);
    } else {
        assert_eq!(outcome.code(), Some(130), "{}", outcome.stderr);
        let report: Value = serde_json::from_str(&outcome.stdout).unwrap();
        assert_eq!(report["state"], "cancelled");
        assert_eq!(report["artifact_created"], already_written);
        let (status, events) = stored_result(&case, report["task_id"].as_str().unwrap());
        assert_eq!(status["recorded_state"], "cancelled");
        assert_eq!(status["artifact_created"], already_written);
        assert_eq!(events.last().unwrap()["event_type"], "cancelled");
        assert!(outcome.stderr.contains("已停止本機任務"));
    }
    assert!(
        await_flag(&disconnected, Duration::from_secs(5)),
        "model connection remained open"
    );
    assert_eq!(artifact.exists(), already_written);
    if already_written {
        let expected = if write_failed { "" } else { "preserve this" };
        assert_eq!(std::fs::read_to_string(&artifact).unwrap(), expected);
    }
}

#[test]
fn interrupt_stops_pending_model_and_preserves_prior_output() {
    let stages = [(false, false), (true, false), (true, true)];
    let cases = stages
        .into_iter()
        .flat_map(|(written, failed)| SIGNALS.map(|signal| (written, failed, signal)));
    run_each(cases, interrupt_case);
}
