use agent_platform_xtask::{repo_root, run_structure, structure_violations};

fn violations(paths: &[&str]) -> Vec<String> {
    let owned: Vec<String> = paths.iter().map(|path| (*path).to_string()).collect();
    structure_violations(&owned)
}

#[test]
fn registered_layout_passes() {
    assert!(
        violations(&[
            "README.md",
            "docs/cli.md",
            "crates/agent/Cargo.toml",
            "crates/agent/src/store/mod.rs",
            "crates/agent/tests/input.rs",
        ])
        .is_empty()
    );
}

#[test]
fn unregistered_top_level_is_rejected() {
    assert_eq!(violations(&["src/main.rs"]).len(), 1);
    assert_eq!(violations(&["notes.md"]).len(), 1);
}

#[test]
fn dump_directories_are_rejected_at_any_depth() {
    assert_eq!(violations(&["crates/agent/src/utils/a.rs"]).len(), 1);
    assert_eq!(violations(&["docs/Misc/a.md"]).len(), 1);
    assert_eq!(violations(&["scripts/shared/a.py"]).len(), 1);
}

#[test]
fn dump_names_are_allowed_as_file_names() {
    assert!(violations(&["crates/agent/src/utils.rs"]).is_empty());
}

#[test]
fn crates_hold_only_crate_directories_with_registered_entries() {
    assert_eq!(violations(&["crates/notes.md"]).len(), 1);
    assert_eq!(violations(&["crates/agent/build.rs"]).len(), 1);
    assert_eq!(violations(&["crates/agent/examples/a.rs"]).len(), 1);
}

#[test]
fn current_repository_passes() {
    assert_eq!(run_structure(&repo_root()).unwrap(), 0);
}
