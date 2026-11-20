use std::collections::BTreeMap;

pub fn empty_files(files: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    files
        .iter()
        .filter(|(_, bytes)| bytes.is_empty())
        .map(|(path, _)| format!("empty file: {path}"))
        .collect()
}
