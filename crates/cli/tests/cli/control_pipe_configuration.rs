use crate::case::Case;
use serde_json::json;

#[test]
fn control_pipe_rejects_invalid_modes_and_cancels_on_input_errors() {
    let case = Case::new();
    let task = case.write_task(&json!({"endpoint":"http://127.0.0.1:9/v1","model":"fixture","task":{"prompt":"work","input":"unused","output":"output.txt","allow_write":true}}));
    for value in ["", "invalid"] {
        let mut command = case.cli(&["run", task.to_str().unwrap()]);
        command.env("AGENT_CONTROL_PIPE", value);
        let result = case.execute(command);
        assert_eq!(result.code(), Some(1));
        assert!(result.stderr.contains("invalid_control_pipe"));
        assert!(!case.state().exists());
    }
    #[cfg(target_os = "linux")]
    {
        use crate::{fake_server::FakeServer, hold_until_disconnect::hold_until_disconnect};
        use std::sync::atomic::AtomicBool;
        let server = FakeServer::start(|_, _, stream| {
            hold_until_disconnect(stream, &AtomicBool::new(false), &AtomicBool::new(false));
        });
        let task = case.write_task(&json!({"endpoint":server.endpoint(),"model":"fixture","task":{"prompt":"work","input":"unused","output":"output.txt","allow_write":true}}));
        let mut command = case.cli(&["run", task.to_str().unwrap()]);
        command
            .env("AGENT_CONTROL_PIPE", "1")
            .stdin(std::fs::File::open(case.root()).unwrap());
        let result = case.execute(command);
        assert_eq!(result.code(), Some(130), "{}", result.stderr);
        assert!(
            result.stderr.contains("control_pipe_failed"),
            "{}",
            result.stderr
        );
        let report: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
        assert_eq!(report["state"], "cancelled");
        assert_eq!(report["artifact_created"], false);
        assert!(!case.root().join("output.txt").exists());
    }
}
