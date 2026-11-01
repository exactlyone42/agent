use agent_harness::{Message, Model, ModelError, ModelReply, ToolCall, ToolSpec, TurnStatus};
use serde_json::{Value, json};
use std::time::Duration;
use url::Url;

pub struct OpenAi {
    http: reqwest::Client,
    url: Url,
    model: String,
    key: String,
}

impl OpenAi {
    pub fn new(
        base: &Url,
        model: impl Into<String>,
        key: impl Into<String>,
        timeout: Duration,
    ) -> Result<Self, ModelError> {
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(timeout)
            .build()
            .map_err(|_| ModelError("無法建立模型連線".into()))?;
        let mut url = base.clone();
        let path = format!("{}/chat/completions", base.path().trim_end_matches('/'));
        url.set_path(&path);
        Ok(Self {
            http,
            url,
            model: model.into(),
            key: key.into(),
        })
    }
}

fn wire_message(message: &Message) -> Value {
    match message {
        Message::User(text) => json!({"role": "user", "content": text}),
        Message::Assistant { text, calls } if calls.is_empty() => {
            json!({"role": "assistant", "content": text})
        }
        Message::Assistant { text, calls } => {
            let calls: Vec<Value> = calls
                .iter()
                .map(|call| {
                    let arguments = call
                        .bad_args
                        .clone()
                        .unwrap_or_else(|| call.args.to_string());
                    json!({
                        "id": call.id,
                        "type": "function",
                        "function": {"name": call.name, "arguments": arguments},
                    })
                })
                .collect();
            let content = if text.is_empty() {
                Value::Null
            } else {
                json!(text)
            };
            json!({"role": "assistant", "content": content, "tool_calls": calls})
        }
        Message::Tool {
            call_id, content, ..
        } => {
            json!({"role": "tool", "tool_call_id": call_id, "content": content})
        }
    }
}

fn wire_tool(spec: &ToolSpec) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": spec.name,
            "description": spec.description,
            "parameters": spec.parameters,
        },
    })
}

fn read_reply(body: &Value) -> Result<ModelReply, ModelError> {
    let bad = || ModelError("模型回應格式不符".into());
    let status = match body
        .pointer("/choices/0/finish_reason")
        .and_then(Value::as_str)
    {
        Some("stop") => TurnStatus::Complete,
        Some("tool_calls") => TurnStatus::ToolCalls,
        Some("length") => TurnStatus::Truncated,
        Some("content_filter") => TurnStatus::Blocked,
        _ => return Err(ModelError("模型停止原因不支援或缺失".into())),
    };
    let message = body.pointer("/choices/0/message").ok_or_else(bad)?;
    let text = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let mut calls = Vec::new();
    for raw in message
        .get("tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = raw.get("id").and_then(Value::as_str).ok_or_else(bad)?;
        let name = raw
            .pointer("/function/name")
            .and_then(Value::as_str)
            .ok_or_else(bad)?;
        let arguments = raw
            .pointer("/function/arguments")
            .and_then(Value::as_str)
            .ok_or_else(bad)?;
        let (args, bad_args) = match serde_json::from_str(arguments) {
            Ok(args) => (args, None),
            Err(_) => (Value::Null, Some(arguments.to_owned())),
        };
        calls.push(ToolCall {
            id: id.to_owned(),
            name: name.to_owned(),
            args,
            bad_args,
        });
    }
    Ok(ModelReply {
        message: Message::Assistant { text, calls },
        status,
    })
}

impl Model for OpenAi {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> Result<ModelReply, ModelError> {
        let mut body = json!({
            "model": self.model,
            "messages": messages.iter().map(wire_message).collect::<Vec<_>>(),
        });
        if !tools.is_empty() {
            body["tools"] = tools.iter().map(wire_tool).collect();
        }
        let mut request = self.http.post(self.url.clone()).json(&body);
        if !self.key.is_empty() {
            request = request.bearer_auth(&self.key);
        }
        let response = request
            .send()
            .await
            .map_err(|_| ModelError("無法連線模型服務".into()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(ModelError(format!("模型服務回應 {status}")));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| ModelError("模型回應不是有效 JSON".into()))?;
        read_reply(&body)
    }
}
