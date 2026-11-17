mod model_failures;
mod wire_keys;

use agent_harness::{Message, Model, ModelError, ToolCall, ToolSpec};
use model_failures::{
    CLIENT_UNAVAILABLE, REPLY_NOT_JSON, REPLY_SHAPE, SERVICE_STATUS, SERVICE_UNREACHABLE,
    TOOL_ARGUMENTS_NOT_JSON,
};
use wire_keys::{
    ARGUMENTS, CALL_ARGUMENTS_POINTER, CALL_NAME_POINTER, CHAT_COMPLETIONS_PATH, CONTENT,
    DESCRIPTION, FIRST_CHOICE_MESSAGE, FUNCTION, ID, MESSAGES, MODEL, NAME, PARAMETERS, ROLE,
    ROLE_ASSISTANT, ROLE_TOOL, ROLE_USER, TOOL_CALL_ID, TOOL_CALLS, TOOLS, TYPE, TYPE_FUNCTION,
};

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
            .map_err(|_| ModelError(CLIENT_UNAVAILABLE.into()))?;
        let mut url = base.clone();
        let path = format!(
            "{}/{CHAT_COMPLETIONS_PATH}",
            base.path().trim_end_matches('/')
        );
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
        Message::User(text) => json!({ROLE: ROLE_USER, CONTENT: text}),
        Message::Assistant { text, calls } if calls.is_empty() => {
            json!({ROLE: ROLE_ASSISTANT, CONTENT: text})
        }
        Message::Assistant { text, calls } => {
            let calls: Vec<Value> = calls
                .iter()
                .map(|call| {
                    json!({
                        ID: call.id,
                        TYPE: TYPE_FUNCTION,
                        FUNCTION: {NAME: call.name, ARGUMENTS: call.args.to_string()},
                    })
                })
                .collect();
            let content = if text.is_empty() {
                Value::Null
            } else {
                json!(text)
            };
            json!({ROLE: ROLE_ASSISTANT, CONTENT: content, TOOL_CALLS: calls})
        }
        Message::Tool {
            call_id, content, ..
        } => {
            json!({ROLE: ROLE_TOOL, TOOL_CALL_ID: call_id, CONTENT: content})
        }
    }
}

fn wire_tool(spec: &ToolSpec) -> Value {
    json!({
        TYPE: TYPE_FUNCTION,
        FUNCTION: {
            NAME: spec.name,
            DESCRIPTION: spec.description,
            PARAMETERS: spec.parameters,
        },
    })
}

fn read_reply(body: &Value) -> Result<Message, ModelError> {
    let bad = || ModelError(REPLY_SHAPE.into());
    let message = body.pointer(FIRST_CHOICE_MESSAGE).ok_or_else(bad)?;
    let text = message
        .get(CONTENT)
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let mut calls = Vec::new();
    for raw in message
        .get(TOOL_CALLS)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let id = raw.get(ID).and_then(Value::as_str).ok_or_else(bad)?;
        let name = raw
            .pointer(CALL_NAME_POINTER)
            .and_then(Value::as_str)
            .ok_or_else(bad)?;
        let arguments = raw
            .pointer(CALL_ARGUMENTS_POINTER)
            .and_then(Value::as_str)
            .ok_or_else(bad)?;
        let args = serde_json::from_str(arguments)
            .map_err(|_| ModelError(TOOL_ARGUMENTS_NOT_JSON.into()))?;
        calls.push(ToolCall {
            id: id.to_owned(),
            name: name.to_owned(),
            args,
        });
    }
    Ok(Message::Assistant { text, calls })
}

impl Model for OpenAi {
    async fn complete(
        &self,
        messages: &[Message],
        tools: &[ToolSpec],
    ) -> Result<Message, ModelError> {
        let mut body = json!({
            MODEL: self.model,
            MESSAGES: messages.iter().map(wire_message).collect::<Vec<_>>(),
        });
        if !tools.is_empty() {
            body[TOOLS] = tools.iter().map(wire_tool).collect();
        }
        let mut request = self.http.post(self.url.clone()).json(&body);
        if !self.key.is_empty() {
            request = request.bearer_auth(&self.key);
        }
        let response = request
            .send()
            .await
            .map_err(|_| ModelError(SERVICE_UNREACHABLE.into()))?;
        let status = response.status();
        if !status.is_success() {
            return Err(ModelError(format!("{SERVICE_STATUS} {status}")));
        }
        let body: Value = response
            .json()
            .await
            .map_err(|_| ModelError(REPLY_NOT_JSON.into()))?;
        read_reply(&body)
    }
}
