use crate::error::XtaskError;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

pub fn copy_snapshot(root: &Path, snapshot: &Path, names: &[String]) -> Result<usize, XtaskError> {
    let root_resolved = root.canonicalize()?;
    let mut copied = 0;
    for name in names
        .iter()
        .filter(|name| !name.is_empty())
        .collect::<BTreeSet<_>>()
    {
        let source = root.join(name);
        if source.symlink_metadata().is_err() {
            continue;
        }
        let resolved = source.canonicalize()?;
        if !resolved.starts_with(&root_resolved) {
            return Err(XtaskError::OutsideLink(name.clone()));
        }
        let target = snapshot.join(name);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, fs::read(&resolved)?)?;
        copied += 1;
    }
    if copied == 0 {
        return Err(XtaskError::EmptySnapshot);
    }
    Ok(copied)
}
