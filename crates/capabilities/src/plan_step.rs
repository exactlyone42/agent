#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    pub id: String,
    pub goal: String,
    pub check: String,
    pub tools: Vec<String>,
    pub done: bool,
}

impl Step {
    pub fn new(id: &str, goal: &str, check: &str) -> Self {
        Self {
            id: id.to_owned(),
            goal: goal.to_owned(),
            check: check.to_owned(),
            tools: Vec::new(),
            done: false,
        }
    }

    pub fn tools(mut self, tools: &[&str]) -> Self {
        self.tools = tools.iter().map(|tool| (*tool).to_owned()).collect();
        self
    }
}
