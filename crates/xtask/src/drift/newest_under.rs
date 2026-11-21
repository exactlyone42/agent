use std::collections::BTreeMap;

const SEPARATOR: char = '/';

pub(crate) fn newest_under(
    files: &BTreeMap<String, Vec<u8>>,
    times: &BTreeMap<String, u64>,
    target: &str,
) -> Option<(String, u64)> {
    let prefix = format!("{target}{SEPARATOR}");
    times
        .iter()
        .filter(|(path, _)| path.as_str() == target || path.starts_with(&prefix))
        .filter(|(path, _)| files.contains_key(path.as_str()))
        .max_by_key(|(_, seconds)| **seconds)
        .map(|(path, seconds)| (path.clone(), *seconds))
}
