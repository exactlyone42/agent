use crate::drift::{
    candidate::Candidate,
    diagram_behind::{CURRENT_DIAGRAMS, diagram_behind},
    doc_behind::doc_behind,
    is_reviewed_doc::is_reviewed_doc,
};
use std::collections::BTreeMap;

pub fn stale_by_age(
    files: &BTreeMap<String, Vec<u8>>,
    times: &BTreeMap<String, u64>,
) -> Vec<Candidate> {
    let docs = files
        .iter()
        .filter(|(path, _)| is_reviewed_doc(path))
        .filter_map(|(path, bytes)| {
            doc_behind(path, &String::from_utf8_lossy(bytes), files, times)
        });
    let diagrams = CURRENT_DIAGRAMS
        .iter()
        .filter_map(|diagram| diagram_behind(diagram, files, times));
    docs.chain(diagrams).collect()
}
