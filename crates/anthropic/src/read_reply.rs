use crate::validate_fields::validate_fields;
use agent_harness::{Message, ModelError, ModelReply, ToolCall, TurnStatus};
use serde_json::Value;

pub(crate) fn read_reply(body: &Value) -> Result<ModelReply, ModelError> {
    let bad = || ModelError("模型回應格式不符".into());
    let status = match body.get("stop_reason").and_then(Value::as_str) {
        Some("end_turn") => TurnStatus::Complete,
        Some("tool_use") => TurnStatus::ToolCalls,
        Some("max_tokens") => TurnStatus::Truncated,
        Some("refusal") => TurnStatus::Blocked,
        Some("pause_turn") => TurnStatus::Continue,
        _ => return Err(ModelError("模型停止原因不支援或缺失".into())),
    };
    let blocks = body
        .get("content")
        .and_then(Value::as_array)
        .ok_or_else(bad)?;
    let mut text = String::new();
    let mut calls = Vec::new();
    for block in blocks {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                validate_fields(block, &["type", "text"])?;
                text.push_str(block.get("text").and_then(Value::as_str).ok_or_else(bad)?);
            }
            Some("tool_use") => {
                validate_fields(block, &["type", "id", "name", "input"])?;
                calls.push(ToolCall {
                    id: block
                        .get("id")
                        .and_then(Value::as_str)
                        .ok_or_else(bad)?
                        .to_owned(),
                    name: block
                        .get("name")
                        .and_then(Value::as_str)
                        .ok_or_else(bad)?
                        .to_owned(),
                    args: block
                        .get("input")
                        .filter(|value| value.is_object())
                        .cloned()
                        .ok_or_else(bad)?,
                    bad_args: None,
                });
            }
            _ => return Err(ModelError("模型回應含有無法保存或續接的內容".into())),
        }
    }
    Ok(ModelReply {
        message: Message::Assistant {
            text,
            calls,
            reasoning: None,
        },
        status,
    })
}
