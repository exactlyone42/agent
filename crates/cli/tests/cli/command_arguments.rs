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
