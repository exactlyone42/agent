use std::fs::Metadata;

#[cfg(unix)]
pub fn single_link(metadata: &Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.nlink() == 1
}

#[cfg(not(unix))]
pub fn single_link(_metadata: &Metadata) -> bool {
    true
}
