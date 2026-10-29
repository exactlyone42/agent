use agent_platform_xtask::validate_reports;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

fn write(root: &Path, coverage: &Value, mutation: &Value) {
    fs::write(root.join("coverage-report.json"), coverage.to_string()).unwrap();
    fs::write(
        root.join("mutation/mutants.out/outcomes.json"),
        mutation.to_string(),
    )
    .unwrap();
}

#[test]
fn empty_or_incomplete_reports_fail() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    fs::create_dir_all(root.join("mutation/mutants.out")).unwrap();
    let coverage = json!({"data": [{"totals": {"lines": {"count": 2, "covered": 1}}}]});
    let mutation =
        json!({"total_mutants": 2, "caught": 1, "unviable": 1, "missed": 0, "timeout": 0});
    write(root, &coverage, &mutation);
    assert!(validate_reports(root).unwrap());
    for (field, value) in [
        ("total_mutants", 0),
        ("caught", 0),
        ("missed", 1),
        ("timeout", 1),
    ] {
        let mut broken = mutation.clone();
        broken[field] = json!(value);
        write(root, &coverage, &broken);
        assert!(!validate_reports(root).unwrap(), "{field}");
    }
    let mut only_unviable = mutation.clone();
    only_unviable["caught"] = json!(0);
    only_unviable["unviable"] = json!(2);
    write(root, &coverage, &only_unviable);
    assert!(!validate_reports(root).unwrap());
    let mut no_lines = coverage.clone();
    no_lines["data"][0]["totals"]["lines"]["count"] = json!(0);
    write(root, &no_lines, &mutation);
    assert!(!validate_reports(root).unwrap());
    let mut uncovered = coverage.clone();
    uncovered["data"][0]["totals"]["lines"]["covered"] = json!(0);
    write(root, &uncovered, &mutation);
    assert!(!validate_reports(root).unwrap());
    write(root, &json!({"data": []}), &mutation);
    assert!(!validate_reports(root).unwrap());
}
