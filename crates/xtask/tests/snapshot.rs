use agent_platform_xtask::{XtaskError, copy_snapshot};
use std::fs;

#[test]
fn empty_snapshot_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("repo");
    let snapshot = directory.path().join("snapshot");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&snapshot).unwrap();
    let result = copy_snapshot(&root, &snapshot, &["missing.txt".to_string()]);
    assert!(matches!(result, Err(XtaskError::EmptySnapshot)));
}

#[cfg(unix)]
#[test]
fn snapshot_scans_internal_links_and_rejects_external_links() {
    use std::os::unix::fs::symlink;
    let directory = tempfile::tempdir().unwrap();
    let base = directory.path();
    let root = base.join("repo");
    let snapshot = base.join("snapshot");
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&snapshot).unwrap();
    fs::write(root.join("source.txt"), "scan this content").unwrap();
    symlink("source.txt", root.join("internal.txt")).unwrap();
    let names = ["internal.txt".to_string(), "deleted.txt".to_string()];
    assert_eq!(copy_snapshot(&root, &snapshot, &names).unwrap(), 1);
    assert_eq!(
        fs::read_to_string(snapshot.join("internal.txt")).unwrap(),
        "scan this content"
    );
    fs::write(base.join("external.txt"), "private outside repo").unwrap();
    symlink(base.join("external.txt"), root.join("external.txt")).unwrap();
    let result = copy_snapshot(&root, &snapshot, &["external.txt".to_string()]);
    assert!(matches!(result, Err(XtaskError::OutsideLink(_))));
    assert!(!snapshot.join("external.txt").exists());
}
