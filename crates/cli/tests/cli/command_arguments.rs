use crate::case::Case;
use crate::run_each::run_each;

#[test]
fn command_arguments() {
    let cases: [(&[&str], i32); 4] = [
        (&["--help"], 0),
        (&["run"], 1),
        (&["other", "missing.json"], 1),
        (&["--help", "extra"], 1),
    ];
    run_each(cases, |(args, expected)| {
        let case = Case::new();
        let result = case.run(args);
        assert_eq!(result.code(), Some(expected));
        let text = if expected == 0 {
            &result.stdout
        } else {
            &result.stderr
        };
        assert!(text.contains("用法："), "{text}");
    });
}

#[test]
fn version_reports_real_contracts_without_opening_state() {
    let case = Case::new();
    let result = case.run(&["version"]);
    assert_eq!(result.code(), Some(0), "{}", result.stderr);
    let version: serde_json::Value = serde_json::from_str(&result.stdout).unwrap();
    assert_eq!(version["schema_version"], 1);
    assert_eq!(version["program_version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(version["error_schema_version"], 1);
    assert!(!case.state().exists());
    let mut store =
        agent_platform_agent::store::TaskStore::open(&case.root().join("reference.sqlite3"))
            .unwrap();
    let (task, _) = store.start().unwrap();
    let event = store.events(&task, 0).unwrap().remove(0);
    assert_eq!(version["event_schema_version"], event.schema_version);
    let connection = crate::open_database::open_database(&case.root().join("reference.sqlite3"));
    let database_version: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version["database_schema_version"], database_version);
    let capabilities = version["capabilities"].as_array().unwrap();
    assert!(capabilities.iter().any(|item| item == "run"));
    assert!(capabilities.iter().any(|item| item == "resume"));
    assert!(!capabilities.iter().any(|item| item == "mcp"));
    std::fs::write(case.state(), b"not a database directory").unwrap();
    let again = case.run(&["version"]);
    assert_eq!(again.code(), Some(0), "{}", again.stderr);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&again.stdout).unwrap(),
        version
    );
    assert_eq!(
        std::fs::read(case.state()).unwrap(),
        b"not a database directory"
    );
    let extra = case.run(&["version", "extra"]);
    assert_eq!(extra.code(), Some(1));
    assert!(extra.stdout.is_empty());
}

#[test]
fn slash_version_matches_command_and_rejects_arguments() {
    let case = Case::new();
    let direct = case.run(&["version"]);
    assert_eq!(direct.code(), Some(0), "{}", direct.stderr);
    let expected: serde_json::Value = serde_json::from_str(&direct.stdout).unwrap();
    let input = case.root().join("commands.txt");
    std::fs::write(&input, b"/version extra\n/version\n/exit\n").unwrap();
    let mut command = case.cli(&[]);
    command.stdin(std::fs::File::open(input).unwrap());
    let result = case.execute(command);
    assert_eq!(result.code(), Some(0), "{}", result.stderr);
    let replies: Vec<serde_json::Value> = result
        .stdout
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert_eq!(replies, vec![expected]);
    assert!(result.stdout.contains("此指令不接受額外參數"));
    assert!(!case.state().exists());
}
