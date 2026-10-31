use crate::all_passed::all_passed;
use crate::error::XtaskError;
use crate::plan::Plan;
use crate::run_secrets::run_secrets;
use crate::run_step::run_step;
use crate::source_hashes::source_hashes;
use crate::validate_reports::validate_reports;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub fn run_checks(root: &Path, evidence: &Path, plan: &Plan) -> Result<i32, XtaskError> {
    println!("本輪原件：{}", evidence.display());
    let before = source_hashes(root)?;
    fs::write(
        evidence.join("source-before.json"),
        serde_json::to_string_pretty(&before)?,
    )?;
    fs::write(
        evidence.join("platform.json"),
        json!({"system": std::env::consts::OS, "machine": std::env::consts::ARCH}).to_string(),
    )?;
    let mut results: BTreeMap<String, i32> = BTreeMap::new();
    for (name, command) in &plan.preflight {
        results.insert(name.clone(), run_step(name, command, root, evidence)?);
    }
    if all_passed(&results) {
        results.insert("secrets".to_string(), run_secrets(root, evidence)?);
        for (name, command) in &plan.main {
            results.insert(name.clone(), run_step(name, command, root, evidence)?);
        }
        let reports = match validate_reports(evidence) {
            Ok(true) => 0,
            Ok(false) => 1,
            Err(error) => {
                fs::write(evidence.join("report-error.txt"), error.to_string())?;
                1
            }
        };
        results.insert("reports".to_string(), reports);
    }
    let after = source_hashes(root)?;
    fs::write(
        evidence.join("source-after.json"),
        serde_json::to_string_pretty(&after)?,
    )?;
    let unchanged = before == after;
    let passed = all_passed(&results) && unchanged;
    let summary = json!({
        "passed": passed,
        "checks": results,
        "source_unchanged": unchanged,
        "scope": "本機工程檢查；不代表 Linux 以外平台、桌面或企業發布驗收",
    });
    fs::write(
        evidence.join("summary.json"),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{summary}");
    Ok(if passed { 0 } else { 1 })
}
