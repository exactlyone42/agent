use crate::collect_lines::collect_lines;
use crate::outcome::Outcome;
use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::thread::{JoinHandle, sleep, spawn};
use std::time::{Duration, Instant};

pub struct Running {
    child: Child,
    lines: Receiver<String>,
    output_lines: Receiver<String>,
    stdout: Option<JoinHandle<String>>,
    stderr: Option<JoinHandle<String>>,
    deadline: Instant,
}

impl Running {
    #[cfg(target_os = "linux")]
    pub fn process_id(&self) -> u32 {
        self.child.id()
    }

    pub fn start(mut command: Command, deadline: Instant) -> Self {
        command.stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command.spawn().expect("cannot start agent-platform");
        let stdout = child.stdout.take().expect("stdout pipe");
        let stderr = child.stderr.take().expect("stderr pipe");
        let (sender, lines) = channel();
        let (output_sender, output_lines) = channel();
        Self {
            child,
            lines,
            output_lines,
            stdout: Some(spawn(move || collect_lines(stdout, Some(output_sender)))),
            stderr: Some(spawn(move || collect_lines(stderr, Some(sender)))),
            deadline,
        }
    }

    fn limit(&self, wait: Duration) -> Instant {
        (Instant::now() + wait).min(self.deadline)
    }

    pub fn exited(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(Some(_)))
    }

    fn wait_until(&mut self, until: Instant) -> bool {
        loop {
            if self.exited() {
                return true;
            }
            if Instant::now() >= until {
                return false;
            }
            sleep(Duration::from_millis(5));
        }
    }

    pub fn stderr_line(&self, wait: Duration) -> Option<String> {
        let until = self.limit(wait);
        self.lines
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .ok()
    }

    pub fn stdout_line(&self, wait: Duration) -> Option<String> {
        let until = self.limit(wait);
        self.output_lines
            .recv_timeout(until.saturating_duration_since(Instant::now()))
            .ok()
    }

    pub fn input(&mut self, line: &str) {
        writeln!(
            self.child.stdin.as_mut().expect("interactive input"),
            "{line}"
        )
        .unwrap();
    }

    pub fn close_input(&mut self) {
        drop(self.child.stdin.take());
    }

    pub fn stop_parent(&mut self) {
        self.child.kill().expect("stop owned interactive parent");
    }

    pub fn wait_for(&mut self, wait: Duration) -> bool {
        let until = self.limit(wait);
        self.wait_until(until)
    }

    pub fn wait_flag(&mut self, flag: &AtomicBool, wait: Duration) -> bool {
        let until = self.limit(wait);
        loop {
            if flag.load(Ordering::SeqCst) {
                return true;
            }
            if self.exited() || Instant::now() >= until {
                return false;
            }
            sleep(Duration::from_millis(10));
        }
    }

    #[cfg(unix)]
    pub fn wait_readable(&mut self, descriptor: std::os::fd::RawFd, wait: Duration) -> bool {
        let until = self.limit(wait);
        loop {
            let mut poll = libc::pollfd {
                fd: descriptor,
                events: libc::POLLIN,
                revents: 0,
            };
            if unsafe { libc::poll(&mut poll, 1, 20) } > 0 {
                return true;
            }
            if self.exited() || Instant::now() >= until {
                return false;
            }
        }
    }

    #[cfg(unix)]
    pub fn signal(&self, signal: i32) {
        unsafe {
            libc::kill(self.child.id() as i32, signal);
        }
    }

    pub fn finish(mut self) -> Outcome {
        let until = self.deadline;
        assert!(
            self.wait_until(until),
            "agent-platform did not finish before the case deadline"
        );
        let status = self.child.wait().expect("wait for agent-platform");
        let stdout = self.stdout.take().map(JoinHandle::join);
        let stderr = self.stderr.take().map(JoinHandle::join);
        Outcome {
            status,
            stdout: stdout.and_then(Result::ok).unwrap_or_default(),
            stderr: stderr.and_then(Result::ok).unwrap_or_default(),
        }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}
