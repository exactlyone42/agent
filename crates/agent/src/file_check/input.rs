use crate::prelude::*;

#[cfg_attr(unix, path = "input/unix.rs")]
#[cfg_attr(windows, path = "input/windows.rs")]
mod sys;

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("the file type, size or identity is not acceptable")]
    InvalidFile,
    #[error("the operating system could not open or check the file")]
    Io(#[from] io::Error),
}

pub fn open_checked(path: &Path, limit: u64) -> Result<File, InputError> {
    let (pinned, before) = sys::pin(path)?;
    ensure_acceptable(&before, limit)?;
    let file = sys::read_options().open(path)?;
    ensure_acceptable(&file.metadata()?, limit)?;
    if !pinned.same_file(&file)? {
        return Err(InputError::InvalidFile);
    }
    Ok(file)
}

fn ensure_acceptable(metadata: &Metadata, limit: u64) -> Result<(), InputError> {
    if metadata.is_file() && metadata.len() <= limit {
        return Ok(());
    }
    Err(InputError::InvalidFile)
}
