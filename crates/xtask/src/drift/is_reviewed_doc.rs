const MARKDOWN_SUFFIX: &str = ".md";
const HISTORY_PREFIXES: [&str; 3] = ["loops/", "docs/run-history/", "docs/standards/records.md"];

pub(crate) fn is_reviewed_doc(path: &str) -> bool {
    path.ends_with(MARKDOWN_SUFFIX)
        && !HISTORY_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix))
}
