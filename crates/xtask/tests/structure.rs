use agent_platform_xtask::{
    Outcome, duplicate_lines, empty_files, locale_literals, read_tracked, repo_root, run_structure,
    structure_violations, unreferenced_docs,
};
use std::collections::BTreeMap;

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
            "sql/store_v1.sql",
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

fn contents(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(path, text)| ((*path).to_string(), text.as_bytes().to_vec()))
        .collect()
}

const LONG_LINE: &str = "這一行夠長而且逐字相同，所以兩份文件都有就算重複記載同一件事。";

#[test]
fn runtime_artifacts_are_rejected_but_cargo_lock_passes() {
    assert_eq!(violations(&["loops/a.checkpoints.jsonl.lock"]).len(), 1);
    assert_eq!(violations(&["crates/agent/tests/run.log"]).len(), 1);
    assert!(violations(&["Cargo.lock"]).is_empty());
}

#[test]
fn scripts_are_allowed_only_in_registered_roots() {
    assert_eq!(violations(&["loops/a.py"]).len(), 1);
    assert_eq!(violations(&["crates/agent/tests/a.sh"]).len(), 1);
    assert_eq!(violations(&["crates/agent/src/store_v1.sql"]).len(), 1);
    assert!(violations(&["scripts/a.py", ".repo-policy/policy.py"]).is_empty());
}

#[test]
fn docs_hold_only_markdown_and_pdf() {
    assert_eq!(violations(&["docs/architecture.svg"]).len(), 1);
    assert_eq!(violations(&["docs/notes"]).len(), 1);
    assert!(violations(&["docs/a.md", "docs/a.pdf"]).is_empty());
}

#[test]
fn empty_files_are_rejected() {
    assert_eq!(
        empty_files(&contents(&[("a.md", ""), ("b.md", "x")])).len(),
        1
    );
    assert!(empty_files(&contents(&[("b.md", "x")])).is_empty());
}

#[test]
fn unreferenced_docs_are_rejected() {
    let files = contents(&[
        ("README.md", "見 a.md 與 c.pdf"),
        ("docs/a.md", "只提到 b.md"),
        ("docs/b.md", "只提到自己 b.md"),
        ("docs/c.pdf", "%PDF"),
        ("docs/d.md", "沒人提到"),
    ]);
    let found = unreferenced_docs(&files);
    assert_eq!(found.len(), 1);
    assert!(found[0].contains("docs/d.md"));
}

#[test]
fn referenced_docs_pass() {
    let files = contents(&[("README.md", "見 a.md"), ("docs/a.md", "內文")]);
    assert!(unreferenced_docs(&files).is_empty());
}

#[test]
fn duplicate_lines_across_documents_are_rejected() {
    let files = contents(&[("README.md", LONG_LINE), ("docs/a.md", LONG_LINE)]);
    assert_eq!(duplicate_lines(&files).len(), 1);
}

#[test]
fn short_table_heading_loop_and_same_file_lines_pass() {
    let table = format!("|{LONG_LINE}|");
    let heading = format!("# {LONG_LINE}");
    let repeated = format!("{LONG_LINE}\n{LONG_LINE}");
    let files = contents(&[
        ("README.md", "太短的行不算"),
        ("docs/a.md", "太短的行不算"),
        ("docs/b.md", &table),
        ("docs/c.md", &table),
        ("docs/d.md", &heading),
        ("docs/e.md", &heading),
        ("docs/f.md", &repeated),
        ("loops/a.md", LONG_LINE),
        ("loops/b.md", LONG_LINE),
    ]);
    assert!(duplicate_lines(&files).is_empty());
}

#[test]
fn locale_specific_text_in_product_source_is_rejected() {
    let files = contents(&[
        ("crates/cli/src/main.rs", "let a = \"用法\";"),
        ("crates/cli/src/command/run.rs", "let a = \"用法：\";"),
        ("crates/cli/src/other.rs", "let a = \"\u{ff1a}\";"),
    ]);
    assert_eq!(locale_literals(&files).len(), 3);
}

#[test]
fn catalogs_tests_docs_and_ascii_source_pass() {
    let files = contents(&[
        ("crates/cli/src/zh_tw_text.rs", "\"用法：\""),
        ("crates/cli/src/zh_tw_store_text.rs", "\"用法：\""),
        ("crates/cli/tests/cli/case.rs", "\"用法：\""),
        ("docs/cli.md", "用法：cargo xtask"),
        ("crates/cli/src/main.rs", "let a = \"usage: \";"),
        ("crates/cli/Cargo.toml", "用法"),
    ]);
    assert!(locale_literals(&files).is_empty());
}

#[test]
fn read_tracked_skips_files_deleted_from_the_worktree() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("a.md"), "x").unwrap();
    let paths = vec!["a.md".to_string(), "gone.md".to_string()];
    let files = read_tracked(directory.path(), &paths).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files.contains_key("a.md"));
}

#[test]
fn current_repository_passes() {
    assert_eq!(run_structure(&repo_root()).unwrap(), Outcome::Passed);
}
