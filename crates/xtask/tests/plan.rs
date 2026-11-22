use agent_platform_xtask::Plan;
use std::path::Path;

fn count(command: &[String], flag: &str) -> usize {
    command.iter().filter(|part| *part == flag).count()
}

#[test]
fn real_plan_lists_every_check_in_order() {
    let evidence = Path::new("/evidence");
    let plan = Plan::real("/bin/xtask", evidence, false);
    let names: Vec<&str> = plan.preflight.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        [
            "gate",
            "versions-rust",
            "versions-coverage",
            "versions-mutation",
            "versions-secrets",
            "versions-dependencies",
        ]
    );
    assert_eq!(plan.preflight[0].1, ["/bin/xtask", "gate"]);
    assert_eq!(plan.preflight[1].1, ["rustc", "--version"]);
    assert_eq!(plan.preflight[4].1, ["gitleaks", "version"]);
    let main: Vec<&str> = plan.main.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(main, ["coverage", "mutation"]);
    let coverage = &plan.main[0].1;
    assert_eq!(coverage[..3], ["cargo", "llvm-cov", "--workspace"]);
    assert_eq!(
        coverage[coverage.len() - 2..],
        ["--output-path", "/evidence/coverage-report.json"]
    );
}

#[test]
fn mutation_command_excludes_input_lines_only_off_windows() {
    let evidence = Path::new("/evidence");
    let unix = Plan::real("x", evidence, false).main[1].1.clone();
    let windows = Plan::real("x", evidence, true).main[1].1.clone();
    assert_eq!(count(&unix, "--exclude-re"), 2);
    assert_eq!(count(&windows, "--exclude-re"), 1);
    for command in [&unix, &windows] {
        assert_eq!(command[..3], ["cargo", "mutants", "--in-place"]);
        assert_eq!(
            command[command.len() - 2..],
            ["--output", "/evidence/mutation"]
        );
        assert!(command.contains(&"--cargo-arg=--locked".to_string()));
    }
    assert!(
        unix.contains(&r"crates/agent/src/file_check/(input|single_link)/windows\.rs:".to_string())
    );
    assert!(
        !windows
            .contains(&r"crates/agent/src/file_check/(input|single_link)/windows\.rs:".to_string())
    );
}
