use crate::prelude::*;
use crate::{
    scan::scan_rules::CATALOG_SUFFIX, scan::scan_rules::LOCALE_SPECIFIC_RANGES,
    scan::scan_rules::RUST_SUFFIX, scan::scan_rules::SOURCE_DIRECTORY,
    scan::scan_rules::SOURCE_DIRECTORY_INDEX,
};

fn is_product_source(path: &str) -> bool {
    let parts: Vec<&str> = path.split(GIT_PATH_SEPARATOR).collect();
    parts.first() == Some(&CRATES_DIRECTORY)
        && parts.get(SOURCE_DIRECTORY_INDEX) == Some(&SOURCE_DIRECTORY)
        && path.ends_with(RUST_SUFFIX)
        && !path.ends_with(CATALOG_SUFFIX)
}

fn is_locale_specific(character: char) -> bool {
    LOCALE_SPECIFIC_RANGES
        .iter()
        .any(|range| range.contains(&character))
}

pub fn locale_literals(files: &BTreeMap<String, Vec<u8>>) -> Vec<String> {
    files
        .iter()
        .filter(|(path, _)| is_product_source(path))
        .filter(|(_, bytes)| {
            String::from_utf8_lossy(bytes)
                .chars()
                .any(is_locale_specific)
        })
        .map(|(path, _)| format!("locale-specific text outside a message catalog: {path}"))
        .collect()
}
