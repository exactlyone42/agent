use crate::evidence::record_keys::PID_KEY;
use crate::prelude::*;

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
    let record = serde_json::json!({PID_KEY: std::process::id(), STARTED_KEY: now_seconds()});
    file.write_all(record.to_string().as_bytes())?;
    drop(file);
    let value = work()?;
    fs::remove_file(marker)?;
    Ok(value)
}
