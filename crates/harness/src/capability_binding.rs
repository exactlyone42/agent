use crate::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityBinding {
    pub name: String,
    pub version: String,
    pub settings: Value,
}

impl CapabilityBinding {
    pub fn new(name: &str, version: &str) -> Self {
        Self {
            name: name.to_owned(),
            version: version.to_owned(),
            settings: Value::Null,
        }
    }

    pub fn settings(mut self, settings: Value) -> Self {
        self.settings = settings;
        self
    }
}
