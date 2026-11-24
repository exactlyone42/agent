use crate::prelude::*;
use crate::scan::scan_rules::ROOT_FROM_MANIFEST;

pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ROOT_FROM_MANIFEST)
}
