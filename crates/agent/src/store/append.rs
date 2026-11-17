use super::{StoreError, TIMESTAMP_NOW, event_kind::EventKind, task_state::TaskState};
use rusqlite::{OptionalExtension, Transaction, params};
use uuid::Uuid;

pub(super) fn append(
    transaction: &Transaction<'_>,
    task_id: &str,
    kind: EventKind,
    state: Option<TaskState>,
    related_id: &str,
) -> Result<(), StoreError> {
    let seq = transaction
        .query_row(
            &format!(
                "UPDATE tasks SET last_seq=last_seq+1, recorded_state=coalesce(?2,recorded_state), updated_at={TIMESTAMP_NOW}
                 WHERE task_id=?1 AND recorded_state IN (?3,?4) RETURNING last_seq"
            ),
            params![task_id, state, TaskState::Running, TaskState::WaitingApproval],
            |r| r.get::<_, i64>("last_seq"),
        )
        .optional()?
        .ok_or(StoreError::Conflict)?;
    transaction.execute(
        "INSERT INTO events(event_id,task_id,seq,event_type,related_id) VALUES (?1,?2,?3,?4,?5)",
        params![Uuid::new_v4().to_string(), task_id, seq, kind, related_id],
    )?;
    Ok(())
}
