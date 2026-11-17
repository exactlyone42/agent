use crate::scan_rules::ROOT_FROM_MANIFEST;
use std::path::PathBuf;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ROOT_FROM_MANIFEST)
}
