use crate::drift::{
    candidate::Candidate, is_reviewed_doc::is_reviewed_doc, missing_in_doc::missing_in_doc,
};
use std::collections::BTreeMap;

pub fn dead_references(files: &BTreeMap<String, Vec<u8>>) -> Vec<Candidate> {
    files
        .iter()
        .filter(|(path, _)| is_reviewed_doc(path))
        .flat_map(|(path, bytes)| missing_in_doc(path, bytes, files))
        .collect()
}
