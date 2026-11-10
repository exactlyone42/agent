use super::{ResultSnapshot, StoreError, recovery_call::RecoveryCall};
use agent_harness::{Message, Session};
use std::collections::HashSet;

pub(super) fn recovery_session(
    snapshot: &ResultSnapshot,
    calls: &[RecoveryCall],
) -> Result<Session, StoreError> {
    let spec = snapshot.spec.as_ref().ok_or(StoreError::InvalidSpec)?;
    if !spec.workspace.is_absolute()
        || !spec.task.input.is_absolute()
        || !spec.task.output.is_absolute()
    {
        return Err(StoreError::InvalidSpec);
    }
    let mut saved = snapshot.session.clone().unwrap_or_else(|| Session {
        revision: 0,
        messages: vec![Message::User(spec.task.prompt.clone())],
        next_turn: 0,
        completed: false,
        unresolved_effect: false,
    });
    if saved.messages.first() != Some(&Message::User(spec.task.prompt.clone())) {
        return Err(StoreError::Unsettled);
    }
    let pending = agent_harness::pending_calls(&saved.messages)?;
    let progress = ["read_input", "write_output"]
        .map(|name| {
            let succeeded = calls
                .iter()
                .filter(|call| call.tool_name == name && call.status == "succeeded")
                .count();
            let unstarted = calls
                .iter()
                .filter(|call| call.tool_name == name && call.unstarted())
                .count()
                + pending
                    .iter()
                    .filter(|call| {
                        call.name == name && calls.iter().all(|saved| saved.call_id != call.id)
                    })
                    .count();
            format!("{name}：成功 {succeeded} 次，未執行 {unstarted} 次")
        })
        .join("；");
    let mut seen = HashSet::new();
    for call in calls {
        if !seen.insert(&call.call_id) {
            return Err(StoreError::Unsettled);
        }
        call.validate(&saved.messages, spec)?;
    }
    let mut resolved = false;
    for message in &mut saved.messages {
        let Message::Tool {
            call_id,
            content,
            is_error,
        } = message
        else {
            continue;
        };
        let call = calls.iter().find(|call| &call.call_id == call_id);
        if !*is_error && call.is_none_or(|call| call.status != "succeeded") {
            return Err(StoreError::Unsettled);
        }
        if *is_error && content == "結果未知" && call.is_none_or(RecoveryCall::unstarted) {
            *content = "未執行".into();
            resolved = true;
        }
    }
    for call in pending {
        if calls
            .iter()
            .find(|saved| saved.call_id == call.id)
            .is_some_and(|call| !call.unstarted())
        {
            return Err(StoreError::Unsettled);
        }
        saved.messages.push(Message::Tool {
            call_id: call.id,
            content: "未執行".into(),
            is_error: true,
        });
        resolved = true;
    }
    if saved.unresolved_effect && !resolved {
        return Err(StoreError::Unsettled);
    }
    saved.messages.push(Message::User(format!(
        "接續原任務。本機回執核對：{progress}。呼叫紀錄不等於執行成功；未執行的呼叫沒有讀到資料或寫入成果，不能當成已完成。請依原任務重新提出尚未完成的動作，不要重做成功寫入。所有新動作仍須批准，這份紀錄不是新批准。"
    )));
    saved.completed = false;
    saved.unresolved_effect = false;
    saved.next_turn = 0;
    saved.revision = 1;
    Ok(saved)
}
