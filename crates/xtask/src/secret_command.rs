use crate::error::XtaskError;
use crate::repo_root::repo_root;
use std::fs;
use std::path::Path;

pub fn secret_command(evidence: &Path) -> Result<Vec<String>, XtaskError> {
    let config = evidence.join("secrets-config.toml");
    fs::write(
        &config,
        fs::read(repo_root().join("scripts/policy/gitleaks.toml"))?,
    )?;
    let ignore = evidence.join("secrets-empty.ignore");
    fs::write(&ignore, "")?;
    let report = evidence.join("secrets-report.json");
    let text = |path: &Path| path.to_string_lossy().into_owned();
    Ok([
        "gitleaks",
        "dir",
        ".",
        "--redact",
        "--no-banner",
        "--config",
        &text(&config),
        "--gitleaks-ignore-path",
        &text(&ignore),
        "--ignore-gitleaks-allow",
        "--report-format",
        "json",
        "--report-path",
        &text(&report),
    ]
    .iter()
    .map(|part| (*part).to_string())
    .collect())
}
