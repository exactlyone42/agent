use crate::prelude::*;
use crate::scan::candidate_lines::candidate_lines;
use crate::scan::scan_rules::{LOOPS_PREFIX, MARKDOWN_SUFFIX, PREVIEW_CHARS};

pub fn duplicate_lines(files: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let mut owners: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
    let documents = files
        .iter()
        .filter(|(path, _)| path.ends_with(MARKDOWN_SUFFIX) && !path.starts_with(LOOPS_PREFIX));
    for (path, bytes) in documents {
        for line in candidate_lines(bytes) {
            owners.entry(line).or_default().insert(path);
        }
    }
    owners
        .iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|(line, paths)| {
            let names: Vec<&str> = paths.iter().copied().collect();
            let head: String = line.chars().take(PREVIEW_CHARS).collect();
            format!(
                "the same text appears in several documents ({}): {head}...",
                names.join(", ")
            )
        })
        .collect()
}
