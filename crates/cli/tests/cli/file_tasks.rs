use crate::file_task_run::FileTaskRun;
use crate::run_each::run_each;

const CASES: [(&str, i32); 18] = [
    ("correct", 0),
    ("wrong", 1),
    ("missing", 1),
    ("denied", 1),
    ("existing", 1),
    ("review", 2),
    ("malformed", 1),
    ("corrupted_spec", 1),
    ("redirect", 1),
    ("existing_equal", 1),
    ("read_limit", 0),
    ("write_limit", 2),
    ("write_oversize", 1),
    ("source_grew", 0),
    ("duplicate_write", 0),
    ("reused_call_id", 0),
    ("receipt_failed", 1),
    ("write_failed", 1),
];

#[test]
fn file_tasks_through_real_cli_and_http() {
    run_each(CASES, |(name, exit_code)| {
        if name == "write_failed" && !cfg!(target_os = "linux") {
            return;
        }
        let run = FileTaskRun::new(name, exit_code);
        let (outcome, requests) = run.execute();
        run.verify(&outcome, &requests);
    });
}
