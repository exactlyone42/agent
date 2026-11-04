use crate::case::Case;
use crate::run_each::run_each;
use serde_json::Value;

#[test]
fn failures_end_with_a_fixed_error_code_line() {
    let cases: [(&[&str], &str); 5] = [
        (&["run"], "usage"),
        (&["approve", "missing", "0"], "invalid_argument"),
        (&["status", "missing"], "state_dir_unavailable"),
        (&["events", "missing", "x"], "invalid_argument"),
        (&["run", "missing.json"], "task_file_unreadable"),
    ];
    run_each(cases, |(args, code)| {
        let case = Case::new();
        let result = case.run(args);
        assert_eq!(result.code(), Some(1));
        let line = result.stderr.lines().last().expect("stderr line");
        let value: Value = serde_json::from_str(line).expect(line);
        assert_eq!(value["error_code"], code, "{line}");
        assert_eq!(value["schema_version"], 1, "{line}");
        assert!(
            value["message"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
        );
    });
}
