use crate::prelude::*;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};

pub const MODE_HINT: &str = "700";

const STATE_DIR_MODE: u32 = 0o700;
const OTHER_ACCESS_MASK: u32 = 0o077;

pub fn create_directory(path: &Path) -> io::Result<()> {
    DirBuilder::new()
        .recursive(true)
        .mode(STATE_DIR_MODE)
        .create(path)
}

pub fn is_private(metadata: &Metadata) -> bool {
    metadata.permissions().mode() & OTHER_ACCESS_MASK == 0
}
