use crate::error::XtaskError;
use crate::text::text;
use std::path::Path;
use std::process::Command;

pub fn chrome_print(html: &Path, pdf: &Path) -> Result<(), XtaskError> {
    let status = Command::new("google-chrome")
        .args([
            "--headless",
            "--disable-gpu",
            "--no-pdf-header-footer",
            "--virtual-time-budget=10000",
        ])
        .arg(format!("--print-to-pdf={}", text(pdf)))
        .arg(format!("file://{}", text(html)))
        .status()?;
    if !status.success() {
        return Err(XtaskError::Report("Chrome 轉 PDF 失敗"));
    }
    Ok(())
}
