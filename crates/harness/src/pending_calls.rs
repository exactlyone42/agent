use crate::{Message, SessionError, ToolCall};
use std::collections::HashSet;

pub fn pending_calls(messages: &[Message]) -> Result<Vec<ToolCall>, SessionError> {
    let mut seen = HashSet::new();
    let mut pending = HashSet::new();
    let mut ordered = Vec::new();
    messages.iter().try_for_each(|message| {
        if let Message::Tool { call_id, .. } = message {
            return pending
                .remove(call_id.as_str())
                .then_some(())
                .ok_or(SessionError::Unsettled);
        }
        if !pending.is_empty() {
            return Err(SessionError::Unsettled);
        }
        if message
            .calls()
            .iter()
            .any(|call| !seen.insert(call.id.as_str()))
        {
            return Err(SessionError::Unsettled);
        }
        pending.extend(message.calls().iter().map(|call| call.id.as_str()));
        ordered.extend(message.calls().iter());
        Ok(())
    })?;
    Ok(ordered
        .into_iter()
        .filter(|call| pending.contains(call.id.as_str()))
        .cloned()
        .collect())
}
