use crate::error::XtaskError;
use crate::structure_violations::structure_violations;
use std::path::Path;
use std::process::Command;

pub fn run_structure(root: &Path) -> Result<i32, XtaskError> {
    println!("檢查目錄結構");
    let listing = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()?;
    if !listing.status.success() || listing.stdout.is_empty() {
        eprintln!("無法列出 Git 追蹤檔案，不能判定目錄結構通過");
        return Ok(1);
    }
    let paths: Vec<String> = String::from_utf8_lossy(&listing.stdout)
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect();
    let violations = structure_violations(&paths);
    for violation in &violations {
        eprintln!("{violation}");
    }
    Ok(i32::from(!violations.is_empty()))
}
