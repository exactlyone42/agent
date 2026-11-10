mod assistant_blocks;
mod push_tool_result;
mod read_reply;
mod validate_fields;
mod wire_messages;
mod wire_tool;

use agent_harness::{Message, ModelError, ModelReply, ToolSpec};
use agent_harness_http::{HttpModel, Wire};
use read_reply::read_reply;
use serde_json::{Value, json};
use std::time::Duration;
use url::Url;
use wire_messages::wire_messages;
use wire_tool::wire_tool;

const API_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 4096;

pub struct Anthropic {
    pub max_tokens: u32,
}

impl Anthropic {
    pub fn new(
        base: &Url,
        model: impl Into<String>,
        key: impl Into<String>,
        timeout: Duration,
    ) -> Result<HttpModel<Self>, ModelError> {
        let wire = Self {
            max_tokens: DEFAULT_MAX_TOKENS,
        };
        HttpModel::new(wire, base, model, key, timeout)
    }

    fn can_replay(message: &Message) -> bool {
        let Message::Assistant {
            reasoning, calls, ..
        } = message
        else {
            return true;
        };
        reasoning.is_none()
            && calls
                .iter()
                .all(|call| call.bad_args.is_none() && call.args.is_object())
    }
}

impl Wire for Anthropic {
    fn path(&self) -> &'static str {
        "messages"
    }

    fn headers(&self, key: &str) -> Vec<(&'static str, String)> {
        vec![
            ("x-api-key", key.to_owned()),
            ("anthropic-version", API_VERSION.to_owned()),
        ]
    }

    fn request(
        &self,
        model: &str,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> Result<Value, ModelError> {
        if !messages.iter().all(Self::can_replay) {
            return Err(ModelError("這個模型入口無法接續已保存的內容".into()));
        }
        let mut body = json!({
            "model": model,
            "max_tokens": self.max_tokens,
            "messages": wire_messages(messages),
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
