use crate::case::Case;
use crate::run_each::run_each;

#[test]
fn invalid_revision_is_rejected_before_opening_database() {
    run_each(["0", "-1", "x"], |revision| {
        let case = Case::new();
        let result = case.run(&["approve", "missing", revision]);
        assert_eq!(result.code(), Some(1));
        assert!(
            result.stderr.contains("必須提供正整數的任務版本"),
            "{}",
            result.stderr
        );
    });
}
