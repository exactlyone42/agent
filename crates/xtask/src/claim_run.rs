use crate::error::XtaskError;
use crate::now_seconds::now_seconds;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn claim_run<T>(
    marker: &Path,
    work: impl FnOnce() -> Result<T, XtaskError>,
) -> Result<T, XtaskError> {
    if let Some(parent) = marker.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(marker)?;
    let record = serde_json::json!({"pid": std::process::id(), "started": now_seconds()});
    file.write_all(record.to_string().as_bytes())?;
    drop(file);
    let value = work()?;
    fs::remove_file(marker)?;
    Ok(value)
}
