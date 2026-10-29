use std::process::ExitStatus;

pub struct Outcome {
    pub status: ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

impl Outcome {
    pub fn code(&self) -> Option<i32> {
        self.status.code()
    }

    #[cfg(unix)]
    pub fn signal(&self) -> Option<i32> {
        use std::os::unix::process::ExitStatusExt;
        self.status.signal()
    }
}
