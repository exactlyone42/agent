use agent_platform_xtask::{
    CandidateKind, dead_references, parse_log, referenced_paths, scan_candidates,
};
use std::collections::BTreeMap;

const DAY: u64 = 86_400;

fn files(entries: &[(&str, &str)]) -> BTreeMap<String, Vec<u8>> {
    entries
        .iter()
        .map(|(path, text)| ((*path).to_string(), text.as_bytes().to_vec()))
        .collect()
}

fn times(entries: &[(&str, u64)]) -> BTreeMap<String, u64> {
    entries
        .iter()
        .map(|(path, seconds)| ((*path).to_string(), *seconds))
        .collect()
}

#[test]
fn log_keeps_the_newest_time_of_each_path() {
    let parsed = parse_log("@300\na.md\nb.md\n\n@200\na.md\n@100\nc.md\n");
    assert_eq!(parsed.get("a.md"), Some(&300));
    assert_eq!(parsed.get("b.md"), Some(&300));
    assert_eq!(parsed.get("c.md"), Some(&100));
}

#[test]
fn backtick_paths_and_links_are_found_with_line_numbers() {
    let text = "intro\nsee `crates/agent/src/lib.rs:12` and [x](../../crates/cli/Cargo.toml)\n";
    let found = referenced_paths("docs/design/a.md", text);
    assert!(found.contains(&(2, "crates/agent/src/lib.rs".to_string())));
    assert!(found.contains(&(2, "crates/cli/Cargo.toml".to_string())));
}

#[test]
fn urls_anchors_commands_and_escapes_are_not_paths() {
    let text = "`cargo xtask gate` [a](https://x.dev/a.md) [b](#top) `../crates/a.rs` `/etc/passwd` `src/lib.rs`\n";
    assert!(referenced_paths("docs/a.md", text).is_empty());
}

#[test]
fn missing_path_is_a_dead_reference_and_existing_ones_are_not() {
    let tracked = files(&[
        (
            "docs/a.md",
            "`crates/agent/src/lib.rs` `crates/gone/x.rs` `crates/agent/src`\n",
        ),
        ("crates/agent/src/lib.rs", "fn a() {}"),
    ]);
    let found = dead_references(&tracked);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].subject, "crates/gone/x.rs");
    assert_eq!(found[0].line, 1);
    assert_eq!(found[0].kind, CandidateKind::DeadReference);
}

#[test]
fn records_and_loops_are_not_reviewed() {
    let tracked = files(&[
        ("loops/t1.md", "`crates/gone/x.rs`\n"),
        ("docs/run-history/a.md", "`crates/gone/x.rs`\n"),
        ("docs/standards/records.md", "`crates/gone/x.rs`\n"),
    ]);
    assert!(dead_references(&tracked).is_empty());
}

#[test]
fn doc_far_behind_its_code_is_flagged_and_near_one_is_not() {
    let tracked = files(&[
        ("docs/a.md", "`crates/agent/src/lib.rs`\n"),
        ("crates/agent/src/lib.rs", "fn a() {}"),
    ]);
    let behind = times(&[
        ("docs/a.md", 100),
        ("crates/agent/src/lib.rs", 100 + 31 * DAY),
    ]);
    let found = scan_candidates(&tracked, &behind);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, CandidateKind::DocBehindCode);
    let near = times(&[
        ("docs/a.md", 100),
        ("crates/agent/src/lib.rs", 100 + 29 * DAY),
    ]);
    assert!(scan_candidates(&tracked, &near).is_empty());
}

#[test]
fn doc_newer_than_its_code_is_not_flagged() {
    let tracked = files(&[
        ("docs/a.md", "`crates/agent/src/lib.rs`\n"),
        ("crates/agent/src/lib.rs", "fn a() {}"),
    ]);
    let fresh = times(&[
        ("docs/a.md", 100 + 90 * DAY),
        ("crates/agent/src/lib.rs", 100),
    ]);
    assert!(scan_candidates(&tracked, &fresh).is_empty());
}

#[test]
fn current_diagram_behind_code_is_flagged_and_marked_program_only() {
    let diagram = "docs/diagrams/current-class-diagram.pdf";
    let tracked = files(&[(diagram, "pdf"), ("crates/agent/src/lib.rs", "fn a() {}")]);
    let behind = times(&[(diagram, 100), ("crates/agent/src/lib.rs", 100 + 15 * DAY)]);
    let found = scan_candidates(&tracked, &behind);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].kind, CandidateKind::DiagramBehindCode);
    assert!(!found[0].kind.needs_ai());
    let near = times(&[(diagram, 100), ("crates/agent/src/lib.rs", 100 + 10 * DAY)]);
    assert!(scan_candidates(&tracked, &near).is_empty());
}
