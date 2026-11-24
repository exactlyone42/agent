use crate::prelude::*;
use std::{
    fs,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
};

pub(super) struct Pinned {
    dev: u64,
    ino: u64,
}

impl Pinned {
    pub(super) fn same_file(&self, file: &File) -> io::Result<bool> {
        let metadata = file.metadata()?;
        Ok(metadata.dev() == self.dev && metadata.ino() == self.ino)
    }
}

pub(super) fn pin(path: &Path) -> io::Result<(Pinned, Metadata)> {
    let metadata = fs::metadata(path)?;
    let pinned = Pinned {
        dev: metadata.dev(),
        ino: metadata.ino(),
    };
    Ok((pinned, metadata))
}

pub(super) fn read_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOCTTY);
    options
}
