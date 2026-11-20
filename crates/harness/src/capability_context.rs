#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityContext {
    pub turn: usize,
    pub pending_tools: usize,
}
