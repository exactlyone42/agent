use crate::case::Case;
use crate::compile_fixture::compile_fixture;
use crate::run_each::run_each;
use serde_json::json;

#[test]
fn source_replaced_after_precheck_is_rejected() {
    let case = Case::new();
    let library = compile_fixture(case.root());
    run_each(["directory", "fifo", "file", "symlink"], |kind| {
        let folder = case.root().join(kind);
        std::fs::create_dir(&folder).unwrap();
        let source = folder.join("input");
        let backup = folder.join("original");
        let replacement = folder.join("other");
        std::fs::write(&source, "unchanged original").unwrap();
        std::fs::write(&replacement, "not authorized").unwrap();
        let task = folder.join("task.json");
        let request = json!({
            "endpoint": "http://127.0.0.1:1/v1",
            "model": "fixture-model",
            "task": {
                "input": source,
                "output": folder.join("output"),
                "prompt": "work",
                "allow_read": true
            }
        });
        std::fs::write(&task, request.to_string()).unwrap();
        let mut command = case.cli(&["run", task.to_str().unwrap()]);
        command
            .env("LD_PRELOAD", &library)
            .env("AGENT_TEST_REPLACE_INPUT", &source)
            .env("AGENT_TEST_ORIGINAL_INPUT", &backup)
            .env("AGENT_TEST_REPLACEMENT_KIND", kind)
            .env("AGENT_TEST_REPLACEMENT_FILE", &replacement);
        let result = case.execute(command);
        assert_eq!(result.code(), Some(1), "{}", result.stderr);
        assert!(result.stderr.contains("來源必須是"), "{}", result.stderr);
        assert_eq!(
            std::fs::read_to_string(&backup).unwrap(),
            "unchanged original"
        );
        assert_eq!(
            std::fs::read_to_string(&replacement).unwrap(),
            "not authorized"
        );
        assert!(!format!("{}{}", result.stdout, result.stderr).contains("not authorized"));
        assert!(!folder.join("output").exists());
    });
}
