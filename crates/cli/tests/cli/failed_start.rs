use crate::case::Case;
use std::process::Stdio;
use std::time::Duration;

#[test]
fn failed_start_keeps_no_current_task_or_database() {
    let case = Case::limited(Duration::from_secs(10));
    let mut command = case.cli(&[]);
    command.stdin(Stdio::piped());
    let mut running = case.spawn(command);
    let greeting = running.stdout_line(Duration::from_secs(5)).unwrap();
    assert!(greeting.contains("互動模式"), "{greeting}");
    let missing = case.root().join("missing-task.json");
    running.input(&format!("/run {}", missing.display()));
    let failure = running.stdout_line(Duration::from_secs(5)).unwrap();
    assert!(failure.starts_with("任務失敗："), "{failure}");
    running.input("/status");
    let status = running.stdout_line(Duration::from_secs(5)).unwrap();
    assert_eq!(status, "尚無目前任務");
    assert!(!case.database().exists());
    running.input("/exit");
    running.close_input();
    let outcome = running.finish();
    assert_eq!(outcome.code(), Some(0), "{}", outcome.stderr);
}
