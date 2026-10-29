use crate::case::Case;
use crate::compile_fixture::compile_fixture;
use crate::keep_open::keep_open;
use crate::stored_result::stored_result;
use crate::task_id_of::task_id_of;
use serde_json::json;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::time::Duration;

#[test]
fn second_interrupt_exits_even_when_file_open_is_stuck() {
    let case = Case::new();
    let library = compile_fixture(case.root());
    let source = case.root().join("source.txt");
    std::fs::write(&source, "preserve source").unwrap();
    let output = case.root().join("output.txt");
    let task = case.write_task(&json!({
        "endpoint": "http://127.0.0.1:1/v1",
        "model": "fixture-model",
        "task": {
            "prompt": "work",
            "input": source,
            "output": output,
            "allow_read": true,
            "allow_write": true
        }
    }));
    let (mut reader, writer) = std::io::pipe().unwrap();
    let mut command = case.cli(&["run", task.to_str().unwrap()]);
    keep_open(&mut command, vec![writer.as_raw_fd()]);
    command
        .env("LD_PRELOAD", &library)
        .env("AGENT_TEST_BLOCK_INPUT", &source)
        .env("AGENT_TEST_BLOCK_NOTIFY_FD", writer.as_raw_fd().to_string());
    let mut running = case.spawn(command);
    drop(writer);
    assert!(
        running.wait_readable(reader.as_raw_fd(), Duration::from_secs(5)),
        "file open did not reach blocking boundary"
    );
    let mut byte = [0u8; 1];
    reader.read_exact(&mut byte).unwrap();
    assert_eq!(&byte, b"1");
    running.signal(libc::SIGINT);
    assert!(!running.wait_for(Duration::from_millis(500)));
    running.signal(libc::SIGINT);
    let outcome = running.finish();
    assert_eq!(outcome.code(), Some(130), "{}", outcome.stderr);
    assert_eq!(outcome.stdout, "");
    assert_eq!(std::fs::read_to_string(&source).unwrap(), "preserve source");
    assert!(!output.exists());
    let (status, events) = stored_result(&case, &task_id_of(&outcome.stderr));
    assert_eq!(status["recorded_state"], "running");
    assert_eq!(events.len(), 1);
}
