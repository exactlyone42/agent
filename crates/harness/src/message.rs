use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
    pub bad_args: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Message {
    User(String),
    Assistant {
        text: String,
        calls: Vec<ToolCall>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning: Option<String>,
    },
    Tool {
        call_id: String,
        content: String,
        is_error: bool,
    },
}

impl Message {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Assistant {
            text: text.into(),
            calls: Vec::new(),
            reasoning: None,
        }
    }

    pub fn call(id: &str, name: &str, args: Value) -> Self {
        let call = ToolCall {
            id: id.into(),
            name: name.into(),
            args,
            bad_args: None,
        };
        Self::Assistant {
            text: String::new(),
            calls: vec![call],
            reasoning: None,
        }
    }

    pub(crate) fn calls(&self) -> &[ToolCall] {
        match self {
            Self::Assistant { calls, .. } => calls,
            _ => &[],
        }
    }
}
