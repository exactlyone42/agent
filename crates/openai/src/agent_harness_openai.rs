mod read_reply;
mod unsupported;
mod validate_fields;
mod wire_message;
mod wire_tool;

use agent_harness::{Message, ModelError, ModelReply, ToolSpec};
use agent_harness_http::{HttpModel, Wire};
use read_reply::read_reply;
use serde_json::{Value, json};
use std::time::Duration;
use url::Url;
use wire_message::wire_message;
use wire_tool::wire_tool;

pub struct OpenAi;

impl OpenAi {
    pub fn new(
        base: &Url,
        model: impl Into<String>,
        key: impl Into<String>,
        timeout: Duration,
    ) -> Result<HttpModel<Self>, ModelError> {
        HttpModel::new(Self, base, model, key, timeout)
    }
}

impl Wire for OpenAi {
    fn path(&self) -> &'static str {
        "chat/completions"
    }

    fn headers(&self, key: &str) -> Vec<(&'static str, String)> {
        if key.is_empty() {
            return Vec::new();
        }
        vec![("authorization", format!("Bearer {key}"))]
    }

    fn request(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> Result<Value, ModelError> {
        let mut body = json!({
            "model": model,
            "messages": messages.iter().map(wire_message).collect::<Vec<_>>(),
        });
        if !tools.is_empty() {
            body["tools"] = tools.iter().map(wire_tool).collect();
        }
        Ok(body)
    }

    fn reply(&self, body: &Value) -> Result<ModelReply, ModelError> {
        read_reply(body)
    }
}
