use crate::error::XtaskError;
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
    let coverage: Value =
        serde_json::from_slice(&fs::read(evidence.join("coverage-report.json"))?)?;
    let mutation: Value = serde_json::from_slice(&fs::read(
        evidence.join("mutation/mutants.out/outcomes.json"),
    )?)?;
    let data = coverage
        .get("data")
        .and_then(Value::as_array)
        .ok_or(XtaskError::Report("data"))?;
    let mut covered_all = !data.is_empty();
    for item in data {
        let lines = item
            .get("totals")
            .and_then(|totals| totals.get("lines"))
            .ok_or(XtaskError::Report("lines"))?;
        covered_all &= number(lines, "count")? > 0 && number(lines, "covered")? > 0;
    }
    let total = number(&mutation, "total_mutants")?;
    let caught = number(&mutation, "caught")?;
    let unviable = number(&mutation, "unviable")?;
    Ok(covered_all
        && caught > 0
        && number(&mutation, "missed")? == 0
        && number(&mutation, "timeout")? == 0
        && caught + unviable == total)
}
