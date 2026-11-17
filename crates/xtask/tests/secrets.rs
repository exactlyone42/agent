mod support;

use agent_platform_xtask::{run_step, secret_command};
use serde_json::{Value, json};
use std::fs;

#[test]
fn cache_exception_does_not_hide_credentials() {
    let value = "0b9f6c1e-3c1c-4f2e-9a57-5f4a3b7c2d11";
    let token = support::canary();
    let cases = [
        ("cache", json!({"prompt_cache_key": value}), true, 0),
        ("other-field", json!({"api_key": value}), true, 1),
        ("outside-path", json!({"prompt_cache_key": value}), false, 1),
        (
            "token-in-cache",
            json!({"prompt_cache_key": token}),
            true,
            1,
        ),
        (
            "same-file-token",
            json!({"prompt_cache_key": value, "api_key": token}),
            true,
            1,
        ),
    ];
    let directory = tempfile::tempdir().unwrap();
    for (name, payload, matching, expected) in cases {
        let snapshot = directory.path().join(name).join("snapshot");
        let evidence = directory.path().join(name).join("evidence");
        fs::create_dir_all(&evidence).unwrap();
        let file = snapshot.join(if matching {
            "docs/evidence/P01/sample/requests.json"
        } else {
            "other/requests.json"
        });
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, payload.to_string()).unwrap();
        let command = secret_command(&evidence).unwrap();
        let code = run_step("secrets", &command, &snapshot, &evidence).unwrap();
        assert_eq!(code, expected, "{name}");
        let log = fs::read_to_string(evidence.join("secrets.txt")).unwrap();
        let report = fs::read_to_string(evidence.join("secrets-report.json")).unwrap();
        assert!(!log.contains(&token));
        assert!(!report.contains(&token));
    }
}

#[test]
fn secret_scanner_detects_canary_and_redacts_it() {
    let directory = tempfile::tempdir().unwrap();
    let snapshot = directory.path().join("snapshot");
    fs::create_dir_all(&snapshot).unwrap();
    for present in [false, true] {
        let evidence = directory.path().join(present.to_string());
        fs::create_dir_all(&evidence).unwrap();
        let canary = support::canary();
        let content = if present {
            format!("token = {canary}")
        } else {
            "ordinary document".to_string()
        };
        fs::write(snapshot.join("sample.txt"), content).unwrap();
        let command = secret_command(&evidence).unwrap();
        let code = run_step("secrets", &command, &snapshot, &evidence).unwrap();
        assert_eq!(code, i32::from(present));
        let log = fs::read_to_string(evidence.join("secrets.txt")).unwrap();
        assert!(!log.contains(&canary));
        let report = fs::read_to_string(evidence.join("secrets-report.json")).unwrap();
        assert!(!report.contains(&canary));
        let findings: Value = serde_json::from_str(&report).unwrap();
        assert_eq!(
            findings.as_array().is_some_and(|list| !list.is_empty()),
            present
        );
    }
}

#[test]
fn scanner_command_keeps_the_flags_that_stop_bypass() {
    let directory = tempfile::tempdir().unwrap();
    let command = secret_command(directory.path()).unwrap();
    let evidence = directory.path().to_string_lossy().into_owned();
    let value_after = |flag: &str| {
        let position = command.iter().position(|part| part == flag).unwrap();
        command[position + 1].clone()
    };
    assert_eq!(&command[..3], ["gitleaks", "dir", "."]);
    for flag in ["--redact", "--no-banner", "--ignore-gitleaks-allow"] {
        assert!(command.iter().any(|part| part == flag), "{flag}");
    }
    assert_eq!(value_after("--report-format"), "json");
    assert!(value_after("--config").starts_with(&evidence));
    assert!(value_after("--gitleaks-ignore-path").starts_with(&evidence));
    assert!(value_after("--report-path").starts_with(&evidence));
    assert_eq!(
        fs::read_to_string(value_after("--gitleaks-ignore-path")).unwrap(),
        ""
    );
}
