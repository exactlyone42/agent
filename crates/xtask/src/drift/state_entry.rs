use serde_json::Value;

pub(crate) const HASH_KEY: &str = "hash";
pub(crate) const REVIEWED_KEY: &str = "reviewed";

pub(crate) fn state_entry(entry: &Value) -> Option<(String, u64)> {
    let hash = entry.get(HASH_KEY)?.as_str()?;
    let reviewed = entry.get(REVIEWED_KEY)?.as_u64()?;
    Some((hash.to_string(), reviewed))
}
