use crate::drift::{evidence::Evidence, verdict::Verdict};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub verdict: Verdict,
    pub reason: String,
    pub evidence: Vec<Evidence>,
}

impl Finding {
    pub fn parse(value: &Value) -> Option<Self> {
        let evidence = value
            .get("evidence")?
            .as_array()?
            .iter()
            .map(Evidence::parse)
            .collect::<Option<Vec<_>>>()?;
        Some(Self {
            verdict: Verdict::parse(value.get("verdict")?.as_str()?)?,
            reason: value.get("reason")?.as_str()?.to_string(),
            evidence,
        })
    }

    pub fn holds(&self, files: &BTreeMap<String, Vec<u8>>) -> bool {
        if self.verdict == Verdict::Ok {
            return true;
        }
        !self.evidence.is_empty() && self.evidence.iter().all(|proof| proof.found_in(files))
    }
}
