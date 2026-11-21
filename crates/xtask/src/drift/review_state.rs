use crate::drift::state_entry::{HASH_KEY, REVIEWED_KEY, state_entry};
use crate::prelude::*;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

pub const REVIEW_INTERVAL_SECONDS: u64 = 28 * 86_400;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReviewState {
    entries: BTreeMap<String, (String, u64)>,
}

impl ReviewState {
    pub fn load(path: &Path) -> Result<Self, XtaskError> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error.into()),
        };
        let value: Value = serde_json::from_slice(&bytes)?;
        let object = value.as_object().ok_or(XtaskError::Report("state"))?;
        let mut entries = BTreeMap::new();
        for (name, entry) in object {
            let parsed = state_entry(entry).ok_or(XtaskError::Report("state entry"))?;
            entries.insert(name.clone(), parsed);
        }
        Ok(Self { entries })
    }

    pub fn save(&self, path: &Path) -> Result<(), XtaskError> {
        let mut object = Map::new();
        for (name, (hash, reviewed)) in &self.entries {
            object.insert(
                name.clone(),
                serde_json::json!({HASH_KEY: hash, REVIEWED_KEY: reviewed}),
            );
        }
        fs::write(path, serde_json::to_string_pretty(&Value::Object(object))?)?;
        Ok(())
    }

    pub fn is_fresh(&self, path: &str, hash: &str, now: u64) -> bool {
        self.entries.get(path).is_some_and(|(known, reviewed)| {
            known == hash && now.saturating_sub(*reviewed) < REVIEW_INTERVAL_SECONDS
        })
    }

    pub fn last_reviewed(&self, path: &str) -> u64 {
        self.entries.get(path).map_or(0, |(_, reviewed)| *reviewed)
    }

    pub fn mark(&mut self, path: &str, hash: &str, now: u64) {
        self.entries
            .insert(path.to_string(), (hash.to_string(), now));
    }
}
