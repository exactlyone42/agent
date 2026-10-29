use super::StoreError;
use rusqlite::{OptionalExtension, Transaction, params};
use uuid::Uuid;

pub(super) fn append(
    transaction: &Transaction<'_>,
    task_id: &str,
    kind: &str,
    state: Option<&str>,
    related_id: &str,
) -> Result<(), StoreError> {
    let seq = transaction.query_row(
        "UPDATE tasks SET last_seq=last_seq+1, recorded_state=coalesce(?2,recorded_state), updated_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE task_id=?1 AND recorded_state IN ('running','waiting_approval') RETURNING last_seq", params![task_id,state], |r| r.get::<_, i64>(0))
        .optional()?.ok_or(StoreError::Conflict)?;
    transaction.execute(
        "INSERT INTO events(event_id,task_id,seq,event_type,related_id) VALUES (?1,?2,?3,?4,?5)",
        params![Uuid::new_v4().to_string(), task_id, seq, kind, related_id],
    )?;
    Ok(())
}
