use crate::structure_violations::DOCS_CATEGORIES;

const DOCS_CATEGORY_INDEX: usize = 1;
const DOCS_CATEGORY_DEPTH: usize = 3;

pub(crate) fn docs_violations(parts: &[&str], path: &str) -> Option<String> {
    let category = parts.get(DOCS_CATEGORY_INDEX)?;
    if parts.len() >= DOCS_CATEGORY_DEPTH && DOCS_CATEGORIES.contains(category) {
        return None;
    }
    Some(format!(
        "docs/ files must sit in a registered category folder: {path}"
    ))
}
