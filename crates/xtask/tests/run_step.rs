use agent_platform_xtask::run_step;

fn rustc(value: &str) -> Vec<String> {
    ["rustc", "--print", value]
        .iter()
        .map(|part| (*part).to_string())
        .collect()
}

#[test]
fn step_records_failure_and_success() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    assert_eq!(run_step("ok", &rustc("sysroot"), root, root).unwrap(), 0);
    let failed = run_step("bad", &rustc("no-such-print-value"), root, root).unwrap();
    assert_ne!(failed, 0);
    for (name, expected) in [("ok", 0), ("bad", failed)] {
        let bytes = std::fs::read(root.join(format!("{name}.json"))).unwrap();
        let record: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(record["exit_code"], expected);
    }
    let missing = vec![root.join("missing-tool").to_string_lossy().into_owned()];
    assert_eq!(run_step("missing", &missing, root, root).unwrap(), 127);
}
