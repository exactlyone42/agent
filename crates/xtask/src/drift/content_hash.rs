use crate::drift::candidate::Candidate;
use sha2::{Digest, Sha256};

pub(crate) fn content_hash(bytes: &[u8], candidates: &[Candidate]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    for candidate in candidates {
        hasher.update(candidate.subject.as_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
