use crate::prelude::*;

pub const MODE_HINT: &str = "owner only";

pub fn create_directory(path: &Path) -> io::Result<()> {
    DirBuilder::new().recursive(true).create(path)
}

pub fn is_private(_metadata: &Metadata) -> bool {
    true
}
