use crate::unsupported::unsupported;
use crate::validate_fields::validate_fields;
use agent_harness::{Message, ModelError, ModelReply, ToolCall, TurnStatus};
use serde_json::Value;

pub(crate) fn read_reply(body: &Value) -> Result<ModelReply, ModelError> {
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
    validate_fields(
        message,
        &["role", "content", "tool_calls", "annotations", "reasoning"],
    )?;
    if message.get("role").is_some_and(|v| v != "assistant")
        || message
            .get("content")
            .is_some_and(|v| !v.is_null() && !v.is_string())
        || message
            .get("tool_calls")
            .is_some_and(|v| !v.is_null() && !v.is_array())
        || message
            .get("reasoning")
            .is_some_and(|v| !v.is_null() && !v.is_string())
        || message
            .get("annotations")
            .is_some_and(|v| !v.is_null() && v.as_array().is_none_or(|a| !a.is_empty()))
    {
        return Err(unsupported());
    }
    let text = message
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_owned();
    let mut calls = Vec::new();
    for (position, raw) in message
        .get("tool_calls")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        validate_fields(raw, &["id", "index", "type", "function"])?;
        let index = u64::try_from(position).map_err(|_| unsupported())?;
        if raw
            .get("index")
            .is_some_and(|value| !value.is_null() && value.as_u64() != Some(index))
        {
            return Err(unsupported());
        }
        if raw.get("type").is_some_and(|v| v != "function") {
            return Err(unsupported());
        }
        validate_fields(raw.get("function").ok_or_else(bad)?, &["name", "arguments"])?;
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
    let reasoning = message
        .get("reasoning")
        .and_then(Value::as_str)
        .map(str::to_owned);
    Ok(ModelReply {
        message: Message::Assistant {
            text,
            calls,
            reasoning,
        },
        status,
    })
}
