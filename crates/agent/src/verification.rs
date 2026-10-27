use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    #[error("無法讀回成果：{0}")]
    Read(#[from] std::io::Error),
    #[error("成果內容不符合任務要求")]
    ContentMismatch,
    #[error("成果必須是一般檔案")]
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
