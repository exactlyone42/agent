use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::MetadataExt,
    },
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskProcess {
    version: u32,
    pub(crate) task_id: String,
    pub(crate) generation: i64,
    pid: u32,
    process_start: u64,
    boot_id: String,
    descriptor: i32,
    #[serde(skip)]
    pidfd: Option<File>,
    #[serde(skip)]
    ownership: Option<File>,
}

impl TaskProcess {
    pub(crate) fn register(ownership: &File, task_id: &str, generation: i64) -> io::Result<()> {
        let metadata = ownership.metadata()?;
        if !metadata.is_file() || metadata.nlink() != 1 || metadata.len() > 4096 {
            return Err(Self::identity_error());
        }
        let pid = std::process::id();
        let record = Self {
            version: 1,
            task_id: task_id.to_owned(),
            generation,
            pid,
            process_start: Self::start_time(pid)?,
            boot_id: Self::read(File::open("/proc/sys/kernel/random/boot_id")?)?
                .trim()
                .to_owned(),
            descriptor: ownership.as_raw_fd(),
            pidfd: None,
            ownership: None,
        };
        let bytes = serde_json::to_vec(&record).map_err(|_| Self::identity_error())?;
        let mut writer = ownership;
        writer.seek(SeekFrom::Start(0))?;
        writer.write_all(&bytes)?;
        ownership.set_len(bytes.len() as u64)?;
        ownership.sync_all()
    }

    pub(crate) fn capture(
        mut ownership: File,
        task_id: &str,
        generation: i64,
    ) -> io::Result<Option<Self>> {
        match ownership.try_lock() {
            Ok(()) => return Ok(None),
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(std::fs::TryLockError::Error(error)) => return Err(error),
        }
        if ownership.metadata()?.len() == 0 {
            return Ok(None);
        }
        ownership.seek(SeekFrom::Start(0))?;
        let json = Self::read(ownership.try_clone()?)?;
        let mut record: Self = serde_json::from_str(&json).map_err(|_| Self::identity_error())?;
        if record.version != 1
            || record.task_id != task_id
            || record.generation != generation
            || record.pid == std::process::id()
        {
            return Err(Self::identity_error());
        }
        let pid = i32::try_from(record.pid).map_err(|_| Self::identity_error())?;
        record.pidfd = Self::open_process(pid)?;
        if record.pidfd.is_none() {
            return Ok(None);
        }
        record.ownership = Some(ownership);
        match record.validate_identity() {
            Ok(()) => Ok(Some(record)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    fn open_process(pid: i32) -> io::Result<Option<File>> {
        let descriptor = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0u32) };
        if descriptor >= 0 {
            return Ok(Some(unsafe { File::from_raw_fd(descriptor as i32) }));
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(None);
        }
        Err(error)
    }

    fn validate_identity(&self) -> io::Result<()> {
        if Self::read(File::open("/proc/sys/kernel/random/boot_id")?)?.trim() != self.boot_id
            || Self::start_time(self.pid)? != self.process_start
        {
            return Err(Self::identity_error());
        }
        let ownership = self
            .ownership
            .as_ref()
            .ok_or_else(Self::identity_error)?
            .metadata()?;
        let live = std::fs::metadata(format!("/proc/{}/fd/{}", self.pid, self.descriptor))?;
        if ownership.dev() != live.dev() || ownership.ino() != live.ino() || ownership.nlink() != 1
        {
            return Err(Self::identity_error());
        }
        let info = Self::read(File::open(format!(
            "/proc/{}/fdinfo/{}",
            self.pid, self.descriptor
        ))?)?;
        let fields: Vec<_> = info
            .lines()
            .find(|line| line.starts_with("lock:"))
            .ok_or_else(Self::identity_error)?
            .split_whitespace()
            .collect();
        if fields.get(2) != Some(&"FLOCK")
            || fields.get(4) != Some(&"WRITE")
            || fields.get(5).and_then(|value| value.parse::<u32>().ok()) != Some(self.pid)
        {
            return Err(Self::identity_error());
        }
        Ok(())
    }

    pub fn terminate(&self) -> io::Result<()> {
        if self.exited()? {
            return Ok(());
        }
        match self.validate_identity() {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        }
        let pidfd = self.pidfd.as_ref().ok_or_else(Self::identity_error)?;
        let result = unsafe {
            libc::syscall(
                libc::SYS_pidfd_send_signal,
                pidfd.as_raw_fd(),
                libc::SIGKILL,
                std::ptr::null::<libc::siginfo_t>(),
                0u32,
            )
        };
        if result == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() == Some(libc::ESRCH) {
            return Ok(());
        }
        Err(error)
    }

    pub fn exited(&self) -> io::Result<bool> {
        let pidfd = self.pidfd.as_ref().ok_or_else(Self::identity_error)?;
        let mut event = libc::pollfd {
            fd: pidfd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        if unsafe { libc::poll(&mut event, 1, 0) } < 0 {
            return Err(io::Error::last_os_error());
        }
        if event.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
            return Err(Self::identity_error());
        }
        Ok(event.revents & (libc::POLLIN | libc::POLLHUP) != 0)
    }

    fn start_time(pid: u32) -> io::Result<u64> {
        let stat = Self::read(File::open(format!("/proc/{pid}/stat"))?)?;
        stat.rsplit_once(')')
            .and_then(|(_, fields)| fields.split_whitespace().nth(19))
            .and_then(|value| value.parse().ok())
            .ok_or_else(Self::identity_error)
    }

    fn read(file: File) -> io::Result<String> {
        let mut text = String::new();
        file.take(4097).read_to_string(&mut text)?;
        if text.len() > 4096 {
            return Err(Self::identity_error());
        }
        Ok(text)
    }

    fn identity_error() -> io::Error {
        io::Error::new(io::ErrorKind::InvalidData, "執行者身分不符，未送出終止訊號")
    }
}
