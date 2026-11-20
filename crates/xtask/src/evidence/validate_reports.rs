use crate::prelude::*;
use crate::{
    evidence::evidence_files::MUTATION_OUTCOMES_FILE, evidence::record_keys::CAUGHT_KEY,
    evidence::record_keys::COUNT_KEY, evidence::record_keys::COVERED_KEY,
    evidence::record_keys::DATA_KEY, evidence::record_keys::LINES_KEY,
    evidence::record_keys::MISSED_KEY, evidence::record_keys::TIMEOUT_KEY,
    evidence::record_keys::TOTAL_MUTANTS_KEY, evidence::record_keys::TOTALS_KEY,
    evidence::record_keys::UNVIABLE_KEY,
};
use serde_json::Value;
use std::fs;
use std::path::Path;

fn number(value: &Value, key: &'static str) -> Result<u64, XtaskError> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or(XtaskError::Report(key))
}

pub fn validate_reports(evidence: &Path) -> Result<bool, XtaskError> {
    let coverage: Value = serde_json::from_slice(&fs::read(evidence.join(COVERAGE_REPORT))?)?;
    let mutation: Value = serde_json::from_slice(&fs::read(
        evidence
            .join(MUTATION_OUTPUT_DIRECTORY)
            .join(MUTATION_OUTCOMES_FILE),
    )?)?;
    let data = coverage
        .get(DATA_KEY)
        .and_then(Value::as_array)
        .ok_or(XtaskError::Report(DATA_KEY))?;
    let mut covered_all = !data.is_empty();
    for item in data {
        let lines = item
            .get(TOTALS_KEY)
            .and_then(|totals| totals.get(LINES_KEY))
            .ok_or(XtaskError::Report(LINES_KEY))?;
        covered_all &= number(lines, COUNT_KEY)? > 0 && number(lines, COVERED_KEY)? > 0;
    }
    let total = number(&mutation, TOTAL_MUTANTS_KEY)?;
    let caught = number(&mutation, CAUGHT_KEY)?;
    let unviable = number(&mutation, UNVIABLE_KEY)?;
    Ok(covered_all
        && caught > 0
        && number(&mutation, MISSED_KEY)? == 0
        && number(&mutation, TIMEOUT_KEY)? == 0
        && caught + unviable == total)
}
