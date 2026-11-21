use serde_json::Value;
use std::collections::BTreeMap;

const MIN_QUOTE_CHARS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub path: String,
    pub line: usize,
    pub quote: String,
}

impl Evidence {
    pub fn parse(value: &Value) -> Option<Self> {
        Some(Self {
            path: value.get("path")?.as_str()?.to_string(),
            line: usize::try_from(value.get("line")?.as_u64()?).ok()?,
            quote: value.get("quote")?.as_str()?.to_string(),
        })
    }

    pub fn found_in(&self, files: &BTreeMap<String, Vec<u8>>) -> bool {
        let quote = self.quote.trim();
        if quote.chars().count() < MIN_QUOTE_CHARS || self.line == 0 {
            return false;
        }
        let Some(bytes) = files.get(&self.path) else {
            return false;
        };
        String::from_utf8_lossy(bytes)
            .lines()
            .nth(self.line - 1)
            .is_some_and(|line| line.contains(quote))
    }
}
