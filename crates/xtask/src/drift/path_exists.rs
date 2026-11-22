use std::collections::BTreeMap;

const SEPARATOR: char = '/';

pub(crate) fn path_exists(files: &BTreeMap<String, Vec<u8>>, path: &str) -> bool {
    if files.contains_key(path) {
        return true;
    }
    let prefix = format!("{path}{SEPARATOR}");
    files
        .range(prefix.clone()..)
        .next()
        .is_some_and(|(name, _)| name.starts_with(&prefix))
}
