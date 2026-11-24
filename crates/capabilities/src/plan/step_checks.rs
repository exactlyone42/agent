use crate::plan::step_check::StepCheck;
use crate::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub struct Checks {
    by_name: BTreeMap<String, Arc<dyn StepCheck>>,
}

impl Checks {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, name: &str, check: impl StepCheck + 'static) -> Self {
        self.by_name.insert(name.to_owned(), Arc::new(check));
        self
    }

    pub fn names(&self) -> Vec<String> {
        self.by_name.keys().cloned().collect()
    }

    pub fn find(&self, name: &str) -> Result<Arc<dyn StepCheck>, PlanError> {
        self.by_name
            .get(name)
            .cloned()
            .ok_or_else(|| PlanError::UnknownCheck {
                name: name.to_owned(),
                available: self.names().join(", "),
            })
    }
}
