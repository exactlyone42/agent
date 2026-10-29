use crate::error::XtaskError;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

fn collect(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), XtaskError> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            collect(&path, files)?;
        } else if path.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

pub fn source_hashes(root: &Path) -> Result<BTreeMap<String, String>, XtaskError> {
    let mut files: Vec<PathBuf> = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]
        .iter()
        .map(|name| root.join(name))
        .collect();
    for directory in ["crates", "scripts", ".cargo"] {
        collect(&root.join(directory), &mut files)?;
    }
    let mut hashes = BTreeMap::new();
    for path in files {
        let digest = Sha256::digest(fs::read(&path)?);
        let name = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .into_owned();
        hashes.insert(
            name,
            digest
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
        );
    }
    Ok(hashes)
}
