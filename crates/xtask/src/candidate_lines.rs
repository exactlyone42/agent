use crate::scan_rules::{MIN_CHARS, SKIPPED_PREFIXES};

pub fn candidate_lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .map(str::trim)
        .filter(|line| line.chars().count() >= MIN_CHARS)
        .filter(|line| {
            !SKIPPED_PREFIXES
                .iter()
                .any(|prefix| line.starts_with(prefix))
        })
        .map(str::to_string)
        .collect()
}
