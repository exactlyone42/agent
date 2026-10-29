#[cfg(windows)]
#[test]
fn windows_read_handle_blocks_changes_until_closed() -> Result<(), Box<dyn std::error::Error>> {
    use agent_platform_agent::input::open_checked;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("agent-input-lock-{}-{stamp}", std::process::id()));
    std::fs::write(&path, "original")?;
    let file = open_checked(&path, 1024)?;
    assert!(std::fs::OpenOptions::new().write(true).open(&path).is_err());
    assert!(std::fs::remove_file(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path)?, "original");
    drop(file);
    std::fs::write(&path, "updated")?;
    assert_eq!(std::fs::read_to_string(&path)?, "updated");
    std::fs::remove_file(path)?;
    Ok(())
}
