use crate::drift::file_group::FileGroup;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    pub chosen: Vec<FileGroup>,
    pub fresh: usize,
    pub deferred: usize,
}
