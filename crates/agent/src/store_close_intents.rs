use crate::prelude::*;

const REVOKE_APPROVALS: &str = "UPDATE approvals SET status=?2 WHERE uses_remaining=?3 AND status IN (?4,?5) AND intent_id IN (SELECT intent_id FROM tool_intents WHERE task_id=?1) RETURNING approval_id";
const MARK_OUTCOME_UNKNOWN: &str =
    "UPDATE tool_intents SET status=?2 WHERE task_id=?1 AND status=?3 RETURNING intent_id";

pub(crate) fn close_intents(
    transaction: &Transaction<'_>,
    task_id: &str,
) -> Result<(), StoreError> {
    let revoke_values: &[&dyn ToSql] = &[
        &task_id,
        &ApprovalStatus::Revoked,
        &USES_AVAILABLE,
        &ApprovalStatus::Pending,
        &ApprovalStatus::Approved,
    ];
    let unknown_values: &[&dyn ToSql] = &[
        &task_id,
        &IntentStatus::OutcomeUnknown,
        &IntentStatus::Dispatched,
    ];
    for (sql, kind, values) in [
        (REVOKE_APPROVALS, EventKind::ApprovalRevoked, revoke_values),
        (
            MARK_OUTCOME_UNKNOWN,
            EventKind::ToolOutcomeUnknown,
            unknown_values,
        ),
    ] {
        let identifiers = {
            let mut statement = transaction.prepare(sql)?;
            statement
                .query_map(values, |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        for id in identifiers {
            let seq: i64 = transaction.query_row(
                "UPDATE tasks SET last_seq=last_seq+1 WHERE task_id=?1 RETURNING last_seq",
                [task_id],
                |r| r.get("last_seq"),
            )?;
            transaction.execute("INSERT INTO events(event_id,task_id,seq,event_type,related_id) VALUES (?1,?2,?3,?4,?5)",params![Uuid::new_v4().to_string(),task_id,seq,kind,id])?;
        }
    }
    Ok(())
}
