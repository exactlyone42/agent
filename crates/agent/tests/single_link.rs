use agent_platform_agent::single_link::single_link;

#[test]
fn plain_file_has_a_single_link() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("single-link-{}", uuid::Uuid::new_v4()));
    std::fs::write(&path, "x")?;
    assert!(single_link(&std::fs::symlink_metadata(&path)?));
    std::fs::remove_file(path)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn hard_linked_file_is_not_single_link() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join(format!("single-link-{}", uuid::Uuid::new_v4()));
    let other = std::env::temp_dir().join(format!("single-link-{}", uuid::Uuid::new_v4()));
    std::fs::write(&path, "x")?;
    std::fs::hard_link(&path, &other)?;
    assert!(!single_link(&std::fs::symlink_metadata(&path)?));
    std::fs::remove_file(path)?;
    std::fs::remove_file(other)?;
    Ok(())
}
