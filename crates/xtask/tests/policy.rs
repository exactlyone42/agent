use agent_platform_xtask::{LINTS, gate_steps, repo_root};
use std::fs;
use std::process::Command;

#[test]
fn gate_enables_tested_lints() {
    let steps = gate_steps(&repo_root());
    let strict = steps
        .iter()
        .find(|step| {
            step.environment
                .iter()
                .any(|(name, _)| name == "CLIPPY_CONF_DIR")
        })
        .unwrap();
    let enabled: Vec<&str> = strict
        .arguments
        .iter()
        .filter_map(|argument| argument.strip_prefix("clippy::"))
        .collect();
    assert_eq!(enabled, LINTS);
}

#[test]
fn compiler_rejects_each_banned_construct() {
    let root = repo_root();
    let driver = Command::new("rustup")
        .args(["which", "clippy-driver"])
        .output()
        .unwrap();
    let driver = String::from_utf8(driver.stdout).unwrap().trim().to_string();
    let fixtures = [
        (
            "valid",
            "pub fn value(x: Option<u8>) -> Result<u8, String> { x.ok_or(\"missing\".into()) }",
        ),
        (
            "unwrap_used",
            "pub fn value(x: Option<u8>) -> u8 { x.unwrap() }",
        ),
        (
            "expect_used",
            "pub fn value(x: Option<u8>) -> u8 { x.expect(\"missing\") }",
        ),
        ("panic", "pub fn value() { panic!(\"failed\") }"),
        ("todo", "pub fn value() { todo!() }"),
        ("unimplemented", "pub fn value() { unimplemented!() }"),
        ("unreachable", "pub fn value() { unreachable!() }"),
        ("assert", "pub fn value(x: bool) { assert!(x); }"),
        (
            "assert_eq",
            "pub fn value(x: u8, y: u8) { assert_eq!(x, y); }",
        ),
        (
            "assert_ne",
            "pub fn value(x: u8, y: u8) { assert_ne!(x, y); }",
        ),
        (
            "debug_assert",
            "pub fn value(x: bool) { debug_assert!(x); }",
        ),
        (
            "debug_assert_eq",
            "pub fn value(x: u8, y: u8) { debug_assert_eq!(x, y); }",
        ),
        (
            "debug_assert_ne",
            "pub fn value(x: u8, y: u8) { debug_assert_ne!(x, y); }",
        ),
        ("mod_module_files", "pub mod nested;"),
        (
            "excessive_nesting",
            "pub fn value(x: u8) -> u8 { let mut s = 0; if x > 0 { s += 1; if x > 1 { s += 1; if x > 2 { s += 1; } } } s }",
        ),
        (
            "redundant_else",
            "pub fn value(x: u8) -> u8 { if x > 0 { return 1; } else { 2 } }",
        ),
    ];
    let directory = tempfile::tempdir().unwrap();
    let workspace = directory.path();
    fs::create_dir_all(workspace.join("nested")).unwrap();
    fs::write(
        workspace.join("nested/mod.rs"),
        "pub fn value() -> u8 { 1 }",
    )
    .unwrap();
    for (name, content) in fixtures {
        let source = workspace.join(format!("{name}.rs"));
        fs::write(&source, content).unwrap();
        let output = Command::new(&driver)
            .args(["--crate-type=lib", "--edition=2024", "--emit=metadata"])
            .arg(&source)
            .arg("-o")
            .arg(workspace.join(format!("{name}.rmeta")))
            .arg("-Dwarnings")
            .args(LINTS.iter().map(|lint| format!("-Dclippy::{lint}")))
            .env("CLIPPY_CONF_DIR", root.join("scripts/policy"))
            .current_dir(workspace)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if name == "valid" {
            assert!(output.status.success(), "{stderr}");
        } else {
            assert!(!output.status.success(), "{name}");
            let lint = if name.contains("assert") {
                "disallowed_macros"
            } else {
                name
            };
            let expected = format!("clippy::{}`", lint.replace('_', "-"));
            assert!(stderr.contains(&expected), "{name}: {stderr}");
        }
    }
}
