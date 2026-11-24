use crate::prelude::*;
use crate::{scan::scan_rules::DOCS_PREFIX, scan::scan_rules::TEXT_SUFFIXES};

pub fn unreferenced_docs(files: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    let texts: Vec<(&String, String)> = files
        .iter()
        .filter(|(path, _)| TEXT_SUFFIXES.iter().any(|suffix| path.ends_with(suffix)))
        .map(|(path, bytes)| (path, String::from_utf8_lossy(bytes).into_owned()))
        .collect();
    files
        .keys()
        .filter(|path| path.starts_with(DOCS_PREFIX))
        .filter(|path| {
            let name = path.rsplit(GIT_PATH_SEPARATOR).next().unwrap_or(path);
            !texts
                .iter()
                .any(|(other, text)| other != path && text.contains(name))
        })
        .map(|path| format!("no document or program refers to it: {path}"))
        .collect()
}
