use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecondOpinion {
    pub agree: bool,
    pub reason: String,
}

impl SecondOpinion {
    pub fn parse(value: &Value) -> Option<Self> {
        Some(Self {
            agree: value.get("agree")?.as_bool()?,
            reason: value.get("reason")?.as_str()?.to_string(),
        })
    }
}
