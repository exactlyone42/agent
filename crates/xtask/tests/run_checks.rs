mod fake_root;
mod support;

use agent_platform_xtask::{Outcome, Plan, run_checks};
use fake_root::{fail, fake_root, pass, read_json, write_reports};
use serde_json::json;
use std::fs;

fn touch(name: &str, path: &str) -> (String, Vec<String>) {
    (
        name.to_string(),
        ["git", "init", "-q", path]
            .iter()
            .map(|part| (*part).to_string())
            .collect(),
    )
}

fn plan(preflight: Vec<(String, Vec<String>)>, main: Vec<(String, Vec<String>)>) -> Plan {
    Plan { preflight, main }
}

#[test]
fn all_steps_pass_and_summary_records_each() {
    let root = fake_root();
    let evidence = tempfile::tempdir().unwrap();
    write_reports(evidence.path(), 3);
    let plan = plan(vec![pass("pre")], vec![pass("coverage"), pass("mutation")]);
    assert_eq!(
        run_checks(root.path(), evidence.path(), &plan).unwrap(),
        Outcome::Passed
    );
    let summary = read_json(&evidence.path().join("summary.json"));
    assert_eq!(summary["passed"], json!(true));
    assert_eq!(summary["source_unchanged"], json!(true));
    let expected = json!({"pre": 0, "secrets": 0, "coverage": 0, "mutation": 0, "reports": 0});
    assert_eq!(summary["checks"], expected);
}

#[test]
fn failed_preflight_skips_the_rest() {
    let root = fake_root();
    let evidence = tempfile::tempdir().unwrap();
    write_reports(evidence.path(), 3);
    let plan = plan(vec![fail("pre")], vec![touch("coverage", "marker")]);
    assert_eq!(
        run_checks(root.path(), evidence.path(), &plan).unwrap(),
        Outcome::Failed
    );
    assert!(!root.path().join("marker").exists());
    let summary = read_json(&evidence.path().join("summary.json"));
    assert_eq!(summary["passed"], json!(false));
    assert_eq!(summary["checks"].as_object().unwrap().len(), 1);
}

#[test]
fn changed_sources_fail_even_when_every_step_passes() {
    let root = fake_root();
    let evidence = tempfile::tempdir().unwrap();
    write_reports(evidence.path(), 3);
    let plan = plan(vec![pass("pre")], vec![touch("coverage", "crates/new")]);
    assert_eq!(
        run_checks(root.path(), evidence.path(), &plan).unwrap(),
        Outcome::Failed
    );
    let summary = read_json(&evidence.path().join("summary.json"));
    assert_eq!(summary["source_unchanged"], json!(false));
    assert_eq!(summary["passed"], json!(false));
    assert_eq!(summary["checks"]["coverage"], json!(0));
}

#[test]
fn missing_or_failing_reports_fail() {
    let root = fake_root();
    let evidence = tempfile::tempdir().unwrap();
    let plan = plan(vec![pass("pre")], vec![pass("coverage")]);
    assert_eq!(
        run_checks(root.path(), evidence.path(), &plan).unwrap(),
        Outcome::Failed
    );
    assert!(evidence.path().join("report-error.txt").exists());
    let broken = tempfile::tempdir().unwrap();
    write_reports(broken.path(), 0);
    let plan = Plan {
        preflight: vec![pass("pre")],
        main: vec![pass("coverage")],
    };
    assert_eq!(
        run_checks(root.path(), broken.path(), &plan).unwrap(),
        Outcome::Failed
    );
    let summary = read_json(&broken.path().join("summary.json"));
    assert_eq!(summary["checks"]["reports"], json!(1));
    assert!(!broken.path().join("report-error.txt").exists());
}

#[test]
fn leaked_credential_fails_the_run() {
    let root = fake_root();
    fs::write(
        root.path().join("leak.txt"),
        format!("GITHUB_TOKEN={}\n", support::canary()),
    )
    .unwrap();
    let evidence = tempfile::tempdir().unwrap();
    write_reports(evidence.path(), 3);
    let plan = plan(vec![pass("pre")], vec![pass("coverage")]);
    assert_eq!(
        run_checks(root.path(), evidence.path(), &plan).unwrap(),
        Outcome::Failed
    );
    let summary = read_json(&evidence.path().join("summary.json"));
    assert_ne!(summary["checks"]["secrets"], json!(0));
}
