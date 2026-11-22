use std::{fs::Metadata, os::unix::fs::MetadataExt};

const SINGLE_LINK_COUNT: u64 = 1;

pub fn single_link(metadata: &Metadata) -> bool {
    metadata.nlink() == SINGLE_LINK_COUNT
}
