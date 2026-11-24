use crate::prelude::*;
use std::fs::{self, OpenOptions};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub struct RunLock {
    marker: PathBuf,
}

impl RunLock {
    pub fn acquire(marker: &Path) -> Result<Self, XtaskError> {
        if let Some(parent) = marker.parent() {
            fs::create_dir_all(parent)?;
        }
        match OpenOptions::new().write(true).create_new(true).open(marker) {
            Ok(_) => Ok(Self {
                marker: marker.to_path_buf(),
            }),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                Err(XtaskError::DriftAlreadyRunning(text(marker)))
            }
            Err(error) => Err(error.into()),
        }
    }
}

impl Drop for RunLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.marker);
    }
}
