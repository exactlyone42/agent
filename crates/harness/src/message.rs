use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Message {
    User(String),
    Assistant {
        text: String,
        calls: Vec<ToolCall>,
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
        }
    }

    pub fn call(id: &str, name: &str, args: Value) -> Self {
        let call = ToolCall {
            id: id.into(),
            name: name.into(),
            args,
        };
        Self::Assistant {
            text: String::new(),
            calls: vec![call],
        }
    }
}
