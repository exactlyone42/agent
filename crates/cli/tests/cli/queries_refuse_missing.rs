use crate::case::Case;
use crate::run_each::run_each;

#[test]
fn queries_refuse_missing_database_and_invalid_arguments() {
    let commands: [&[&str]; 9] = [
        &["status", "missing"],
        &["events", "missing"],
        &["events", "missing", "x"],
        &["approve", "missing", "1"],
        &["deny", "missing", "1"],
        &["approvals", "missing"],
        &["events", "missing", "9223372036854775808"],
        &["status"],
        &["events", "id", "0", "extra"],
    ];
    run_each(commands, |command| {
        let case = Case::new();
        let result = case.run(command);
        assert_eq!(result.code(), Some(1));
        assert_eq!(result.stdout, "");
        assert!(!case.state().exists());
    });
}
