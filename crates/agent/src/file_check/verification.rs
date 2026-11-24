use crate::prelude::*;

#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("could not read the artifact back: {0}")]
    Read(#[from] std::io::Error),
    #[error("the artifact content does not match the expectation")]
    ContentMismatch,
    #[error("the artifact must be a regular file")]
    UnsupportedFile,
}

pub fn verify_file(file: &mut File, expected: &[u8]) -> Result<(), VerificationError> {
    if !file.metadata()?.is_file() {
        return Err(VerificationError::UnsupportedFile);
    }
    file.seek(SeekFrom::Start(0))?;
    let mut actual = Vec::new();
    file.take(expected.len() as u64 + 1)
        .read_to_end(&mut actual)?;
    if actual != expected {
        return Err(VerificationError::ContentMismatch);
    }
    Ok(())
}
