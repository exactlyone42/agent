use crate::keep_open::keep_open;
use crate::running::Running;
use std::io::{PipeReader, PipeWriter, Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub struct SyncPipes {
    notify_read: PipeReader,
    _notify_write: PipeWriter,
    _release_read: PipeReader,
    release_write: PipeWriter,
}

impl SyncPipes {
    pub fn attach(command: &mut Command, library: &Path, artifact: &Path) -> Self {
        let (notify_read, notify_write) = std::io::pipe().expect("notify pipe");
        let (release_read, release_write) = std::io::pipe().expect("release pipe");
        let notify = notify_write.as_raw_fd();
        let release = release_read.as_raw_fd();
        keep_open(command, vec![notify, release]);
        command
            .env("LD_PRELOAD", library)
            .env("AGENT_TEST_BLOCK_SYNC", artifact)
            .env("AGENT_TEST_SYNC_NOTIFY", notify.to_string())
            .env("AGENT_TEST_SYNC_RELEASE", release.to_string());
        Self {
            notify_read,
            _notify_write: notify_write,
            _release_read: release_read,
            release_write,
        }
    }

    pub fn reached(&mut self, running: &mut Running) -> bool {
        if !running.wait_readable(self.notify_read.as_raw_fd(), Duration::from_secs(5)) {
            return false;
        }
        let mut byte = [0u8; 1];
        self.notify_read.read_exact(&mut byte).is_ok() && byte == *b"1"
    }

    pub fn release(&mut self) {
        self.release_write.write_all(b"1").expect("release sync");
    }
}
