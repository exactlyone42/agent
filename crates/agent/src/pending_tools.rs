use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct PendingTools {
    ids: HashSet<String>,
}

impl PendingTools {
    pub fn expect(&mut self, call_id: &str) -> bool {
        self.ids.insert(call_id.to_owned())
    }

    pub fn report(&mut self, call_id: &str) -> bool {
        self.ids.remove(call_id)
    }

    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }
}
