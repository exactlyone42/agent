use crate::case::Case;

#[test]
fn existing_directory_without_database_is_refused_for_queries() {
    let case = Case::new();
    create_private_directory(&case.state());
    let result = case.run(&["status", "missing"]);
    assert_eq!(result.code(), Some(1));
    assert!(
        result.stderr.contains("找不到或無法讀取任務資料庫"),
        "{}",
        result.stderr
    );
    assert!(!case.database().exists());
}

fn create_private_directory(path: &std::path::Path) {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).expect("create state directory");
}
