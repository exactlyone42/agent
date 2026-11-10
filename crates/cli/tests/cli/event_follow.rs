use crate::case::Case;
use agent_platform_agent::store::TaskStore;
use std::io::{BufRead, BufReader};
use std::process::{Child, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

struct EventReader {
    child: Child,
    lines: Receiver<String>,
}

impl EventReader {
    fn start(case: &Case, task: &str, after: Option<&str>) -> Self {
        let mut args = vec!["events", task];
        args.extend(after);
        args.push("--follow");
        let mut child = case
            .cli(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let pipe = child.stdout.take().unwrap();
        let (send, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines() {
                if send.send(line.unwrap()).is_err() {
                    return;
                }
            }
        });
        Self { child, lines }
    }

    fn next(&self) -> i64 {
        let line = self
            .lines
            .recv_timeout(Duration::from_secs(5))
            .expect("live JSON event before exit");
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        value["seq"].as_i64().unwrap()
    }

    #[cfg(unix)]
    fn interrupt(&mut self) {
        unsafe {
            libc::kill(self.child.id() as i32, libc::SIGINT);
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = self.child.try_wait().unwrap() {
                assert!(status.success());
                return;
            }
            assert!(std::time::Instant::now() < deadline, "follow must stop");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for EventReader {
    fn drop(&mut self) {
        if self.child.try_wait().unwrap().is_none() {
            self.child.kill().unwrap();
            self.child.wait().unwrap();
        }
    }
}

#[test]
fn event_follow_pages_live_updates_and_reconnect_without_loss_or_duplicates() {
    let case = Case::new();
    std::fs::create_dir(case.state()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(case.state(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut store = TaskStore::open(&case.database()).unwrap();
    let (task, _) = store.start().unwrap();
    let connection = rusqlite::Connection::open(case.database()).unwrap();
    for seq in 2..=205 {
        connection.execute("INSERT INTO events(event_id, task_id, seq, event_type) VALUES (?1, ?2, ?3, 'started')", rusqlite::params![format!("event-{seq}"), task, seq]).unwrap();
    }
    let mut reader = EventReader::start(&case, &task, None);
    for seq in 1..=205 {
        assert_eq!(reader.next(), seq);
    }
    assert!(
        reader
            .lines
            .recv_timeout(Duration::from_millis(300))
            .is_err()
    );
    connection.execute("INSERT INTO events(event_id, task_id, seq, event_type) VALUES ('live', ?1, 206, 'started')", [&task]).unwrap();
    assert_eq!(reader.next(), 206);
    #[cfg(unix)]
    reader.interrupt();
    drop(reader);
    let record = store.status(&task).unwrap();
    assert_eq!(record.recorded_state, "running");
    assert_eq!(store.events(&task, 205).unwrap().len(), 1);
    let mut resumed = EventReader::start(&case, &task, Some("206"));
    connection.execute("INSERT INTO events(event_id, task_id, seq, event_type) VALUES ('resumed', ?1, 207, 'started')", [&task]).unwrap();
    assert_eq!(resumed.next(), 207);
    assert!(
        resumed
            .lines
            .recv_timeout(Duration::from_millis(300))
            .is_err()
    );
    #[cfg(unix)]
    resumed.interrupt();
    drop(resumed);
    let single = case.run(&["events", &task, "0"]);
    assert_eq!(single.code(), Some(0));
    assert_eq!(single.stdout.lines().count(), 100);
    #[cfg(target_os = "linux")]
    {
        let mut command = case.cli(&["events", &task, "--follow"]);
        command.stdout(
            std::fs::OpenOptions::new()
                .write(true)
                .open("/dev/full")
                .unwrap(),
        );
        let output = command.output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("output_failed")
        );
    }
}

#[test]
fn event_follow_rejects_invalid_options_and_missing_tasks() {
    let case = Case::new();
    for args in [
        vec!["events", "missing", "--follow"],
        vec!["events", "missing", "-1", "--follow"],
        vec!["events", "missing", "x", "--follow"],
        vec!["events", "missing", "0", "--follow", "extra"],
        vec!["events", "missing", "--follow", "--follow"],
    ] {
        let result = case.run(&args);
        assert_eq!(result.code(), Some(1));
        assert!(result.stdout.is_empty());
        assert!(!case.state().exists());
    }
    std::fs::create_dir(case.state()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(case.state(), std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    let mut store = TaskStore::open(&case.database()).unwrap();
    let (task, _) = store.start().unwrap();
    let missing = case.run(&["events", "missing", "--follow"]);
    assert_eq!(missing.code(), Some(1));
    assert!(missing.stderr.contains("task_not_found"));
    for cursor in ["-1", "x", "9223372036854775808"] {
        let result = case.run(&["events", &task, cursor, "--follow"]);
        assert_eq!(result.code(), Some(1));
        assert!(result.stderr.contains("invalid_argument"));
        assert!(result.stdout.is_empty());
    }
}
