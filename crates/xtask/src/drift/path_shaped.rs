const ROOTS: [&str; 7] = [
    "crates/",
    "docs/",
    "scripts/",
    "sql/",
    "loops/",
    ".cargo/",
    ".repo-policy/",
];
const PATH_EXTRA_CHARS: &str = "_./-";
const SEPARATOR: char = '/';
const PARENT: &str = "..";

pub(crate) fn path_shaped(span: &str) -> Option<String> {
    let trimmed = span.trim_end_matches(SEPARATOR);
    let without_line = match trimmed.rsplit_once(':') {
        Some((head, tail)) if !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) => head,
        _ => trimmed,
    };
    let allowed = without_line
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || PATH_EXTRA_CHARS.contains(c));
    let rooted = ROOTS.iter().any(|root| without_line.starts_with(root));
    (allowed && rooted && !without_line.contains(PARENT)).then(|| without_line.to_string())
}
