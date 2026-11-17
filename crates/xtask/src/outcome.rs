#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    Failed,
}

impl Outcome {
    pub fn judge(passed: bool) -> Self {
        if passed { Self::Passed } else { Self::Failed }
    }
}
