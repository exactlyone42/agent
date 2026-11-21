#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Ok,
    Update,
    Delete,
}

impl Verdict {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "ok" => Some(Self::Ok),
            "update" => Some(Self::Update),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Update => "update",
            Self::Delete => "delete",
        }
    }
}
