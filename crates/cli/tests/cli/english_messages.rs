use crate::case::{Case, ENGLISH, LANGUAGE_ENV};
use crate::run_each::run_each;

#[test]
fn english_users_get_no_chinese_text() {
    let cases: [(&[&str], Option<&str>, &str); 3] = [
        (&["approve", "missing", "0"], None, "positive integer"),
        (&["status", "missing"], Some(""), "must not be empty"),
        (&["events", "missing", "x"], None, "must be an integer"),
    ];
    run_each(cases, |(args, state_dir, expected)| {
        let case = Case::new();
        let mut command = case.cli(args);
        command.env(LANGUAGE_ENV, ENGLISH);
        if let Some(value) = state_dir {
            command.env("AGENT_STATE_DIR", value);
        }
        let result = case.execute(command);
        assert_eq!(result.code(), Some(1));
        assert!(result.stderr.contains(expected), "{}", result.stderr);
        assert!(result.stderr.is_ascii(), "{}", result.stderr);
    });
}
