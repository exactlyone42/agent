use super::StoreError;
use rusqlite::{Transaction, params};
use uuid::Uuid;

pub(super) fn close_intents(
    transaction: &Transaction<'_>,
    task_id: &str,
) -> Result<(), StoreError> {
    for (sql, kind) in [
        (
            "UPDATE approvals SET status='revoked' WHERE uses_remaining=1 AND status IN ('pending','approved') AND intent_id IN (SELECT intent_id FROM tool_intents WHERE task_id=?1) RETURNING approval_id",
            "approval_revoked",
        ),
        (
            "UPDATE tool_intents SET status='outcome_unknown' WHERE task_id=?1 AND status='dispatched' RETURNING intent_id",
            "tool_outcome_unknown",
        ),
    ] {
        let identifiers = {
            let mut statement = transaction.prepare(sql)?;
            statement
                .query_map([task_id], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        for id in identifiers {
            let seq: i64 = transaction.query_row(
                "UPDATE tasks SET last_seq=last_seq+1 WHERE task_id=?1 RETURNING last_seq",
                [task_id],
                |r| r.get(0),
            )?;
            transaction.execute("INSERT INTO events(event_id,task_id,seq,event_type,related_id) VALUES (?1,?2,?3,?4,?5)",params![Uuid::new_v4().to_string(),task_id,seq,kind,id])?;
        }
    }
    Ok(())
}
