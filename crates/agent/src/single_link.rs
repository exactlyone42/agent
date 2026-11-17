#[cfg(unix)]
use crate::task_settings::SINGLE_LINK_COUNT;
use std::fs::Metadata;

#[cfg(unix)]
pub fn single_link(metadata: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() == SINGLE_LINK_COUNT
}

#[cfg(not(unix))]
pub fn single_link(_metadata: &Metadata) -> bool {
    true
}
