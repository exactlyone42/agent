use crate::prelude::*;
use crate::{
    evidence::evidence_files::PLATFORM_FILE, evidence::evidence_files::REPORT_ERROR_FILE,
    evidence::evidence_files::SOURCE_AFTER_FILE, evidence::evidence_files::SOURCE_BEFORE_FILE,
    evidence::evidence_files::SUMMARY_FILE, evidence::record_keys::CHECKS_KEY,
    evidence::record_keys::MACHINE_KEY, evidence::record_keys::PASSED_KEY,
    evidence::record_keys::SCOPE_KEY, evidence::record_keys::SOURCE_UNCHANGED_KEY,
    evidence::record_keys::SYSTEM_KEY, evidence::source_hashes::source_hashes,
    evidence::validate_reports::validate_reports, run::all_passed::all_passed,
    run::run_secrets::run_secrets, run::step_names::REPORTS_STEP,
};

pub fn run_checks(root: &Path, evidence: &Path, plan: &Plan) -> Result<Outcome, XtaskError> {
    println!("evidence for this run: {}", evidence.display());
    let before = source_hashes(root)?;
    fs::write(
        evidence.join(SOURCE_BEFORE_FILE),
        serde_json::to_string_pretty(&before)?,
    )?;
    fs::write(
        evidence.join(PLATFORM_FILE),
        json!({SYSTEM_KEY: std::env::consts::OS, MACHINE_KEY: std::env::consts::ARCH}).to_string(),
    )?;
    let mut results: BTreeMap<String, i32> = BTreeMap::new();
    for (name, command) in &plan.preflight {
        results.insert(name.clone(), run_step(name, command, root, evidence)?);
    }
    if all_passed(&results) {
        results.insert(SECRETS_STEP.to_string(), run_secrets(root, evidence)?);
        for (name, command) in &plan.main {
            results.insert(name.clone(), run_step(name, command, root, evidence)?);
        }
        let reports = match validate_reports(evidence) {
            Ok(true) => 0,
            Ok(false) => FAILURE_CODE,
            Err(error) => {
                fs::write(evidence.join(REPORT_ERROR_FILE), error.to_string())?;
                FAILURE_CODE
            }
        };
        results.insert(REPORTS_STEP.to_string(), reports);
    }
    let after = source_hashes(root)?;
    fs::write(
        evidence.join(SOURCE_AFTER_FILE),
        serde_json::to_string_pretty(&after)?,
    )?;
    let unchanged = before == after;
    let passed = all_passed(&results) && unchanged;
    let summary = json!({
        PASSED_KEY: passed,
        CHECKS_KEY: results,
        SOURCE_UNCHANGED_KEY: unchanged,
        SCOPE_KEY: "local engineering checks; not acceptance on three platforms, desktop or enterprise release",
    });
    fs::write(
        evidence.join(SUMMARY_FILE),
        serde_json::to_string_pretty(&summary)?,
    )?;
    println!("{summary}");
    Ok(Outcome::judge(passed))
}
