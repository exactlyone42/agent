use crate::prelude::*;
use crate::{
    scan::structure_violations::CARGO_DIRECTORY, scan::structure_violations::CARGO_TOML,
    scan::structure_violations::TOOLCHAIN_FILE,
};
use std::ops::RangeInclusive;

pub(crate) const MIN_CHARS: usize = 30;
pub(crate) const SKIPPED_PREFIXES: [&str; 5] = ["|", "#", "```", "- [", "!["];
pub(crate) const MARKDOWN_SUFFIX: &str = ".md";
pub(crate) const LOOPS_PREFIX: &str = "loops/";
pub(crate) const PREVIEW_CHARS: usize = 24;
pub(crate) const DOCS_PREFIX: &str = "docs/";
pub(crate) const TEXT_SUFFIXES: [&str; 5] = [MARKDOWN_SUFFIX, ".json", ".py", ".rs", ".toml"];
pub(crate) const SOURCE_DIRECTORY: &str = "src";
pub(crate) const RUST_SUFFIX: &str = ".rs";
pub(crate) const CATALOG_SUFFIX: &str = "_text.rs";
pub(crate) const SOURCE_DIRECTORY_INDEX: usize = 2;
pub(crate) const LOCALE_SPECIFIC_RANGES: [RangeInclusive<char>; 3] = [
    '\u{2E80}'..='\u{9FFF}',
    '\u{F900}'..='\u{FAFF}',
    '\u{FF00}'..='\u{FFEF}',
];
pub(crate) const ROOT_FILES: [&str; 3] = [CARGO_TOML, CARGO_LOCK, TOOLCHAIN_FILE];
pub(crate) const SOURCE_DIRECTORIES: [&str; 3] =
    [CRATES_DIRECTORY, SCRIPTS_DIRECTORY, CARGO_DIRECTORY];
pub(crate) const ROOT_FROM_MANIFEST: &str = "../..";
