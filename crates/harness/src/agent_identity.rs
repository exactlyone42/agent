#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentId(pub String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentIdentity {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Goal {
    pub text: String,
}
