use crate::prelude::*;
use crate::{
    evidence::evidence_files::CONFIG_FILE, evidence::evidence_files::IGNORE_FILE,
    evidence::evidence_files::POLICY_CONFIG_FILE, evidence::evidence_files::REPORT_FILE,
    repo_root::repo_root, run::scanner_arguments::CONFIG_FLAG,
    run::scanner_arguments::IGNORE_ALLOW_COMMENTS_FLAG, run::scanner_arguments::IGNORE_PATH_FLAG,
    run::scanner_arguments::NO_BANNER_FLAG, run::scanner_arguments::REDACT_FLAG,
    run::scanner_arguments::REPORT_FORMAT_FLAG, run::scanner_arguments::REPORT_FORMAT_JSON,
    run::scanner_arguments::REPORT_PATH_FLAG, run::scanner_arguments::SCAN_DIRECTORY_COMMAND,
    run::scanner_arguments::SCAN_TARGET,
};
use std::fs;
use std::path::Path;

pub fn secret_command(evidence: &Path) -> Result<Vec<String>, XtaskError> {
    let config = evidence.join(CONFIG_FILE);
    fs::write(
        &config,
        fs::read(repo_root().join(POLICY_DIRECTORY).join(POLICY_CONFIG_FILE))?,
    )?;
    let ignore = evidence.join(IGNORE_FILE);
    fs::write(&ignore, "")?;
    let report = evidence.join(REPORT_FILE);
    let text = |path: &Path| path.to_string_lossy().into_owned();
    Ok([
        GITLEAKS,
        SCAN_DIRECTORY_COMMAND,
        SCAN_TARGET,
        REDACT_FLAG,
        NO_BANNER_FLAG,
        CONFIG_FLAG,
        &text(&config),
        IGNORE_PATH_FLAG,
        &text(&ignore),
        IGNORE_ALLOW_COMMENTS_FLAG,
        REPORT_FORMAT_FLAG,
        REPORT_FORMAT_JSON,
        REPORT_PATH_FLAG,
        &text(&report),
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect())
}
