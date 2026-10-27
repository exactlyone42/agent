use super::{StoreError, TaskStore};
use crate::task::{FILE_LIMIT, FileTask, ReadArgs, WriteArgs};
use rusqlite::{OptionalExtension, Transaction, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use uuid::Uuid;

pub const APPROVAL_TTL_SECONDS: i64 = 120;
pub const TOOL_CALL_LIMIT: i64 = 64;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    #[default]
    Preapproved,
    Manual,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpec {
    pub workspace: PathBuf,
    pub endpoint: String,
    pub model: String,
    pub task: FileTask,
    pub approval_mode: ApprovalMode,
}

#[derive(Debug, Serialize)]
pub struct ApprovalRecord {
    pub intent_id: String,
    pub intent_status: String,
    pub step_id: i64,
    pub approval_id: String,
    pub task_id: String,
    pub spec_revision: i64,
    pub policy_revision: i64,
    pub tool_name: String,
    pub tool_version: i64,
    pub scope_path: String,
    pub args_json: String,
    pub args_hash: String,
    pub status: String,
    pub uses_remaining: i64,
    pub expires_at: i64,
    pub decision_source: String,
}

pub struct PreparedIntent {
    pub intent_id: String,
    pub approval_id: String,
    pub task_id: String,
    pub call_id: String,
    pub args_hash: String,
    pub spec_revision: i64,
}

#[derive(Debug, PartialEq)]
pub enum Claim {
    Pending,
    Granted,
    Denied,
}

pub(super) fn digest(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

fn append(
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

impl TaskStore {
    pub fn prepare_intent(
        &mut self,
        task_id: &str,
        step_id: i64,
        call_id: &str,
        tool_name: &str,
        args: &str,
    ) -> Result<PreparedIntent, StoreError> {
        if step_id < 0 {
            return Err(StoreError::InvalidSpec);
        }
        if call_id.is_empty() || call_id.len() > 256 || args.len() > 6 * FILE_LIMIT as usize + 1024
        {
            return Err(StoreError::InvalidToolCall);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (spec_json, spec_hash, spec_revision, policy_revision, generation): (String,String,i64,i64,i64) = transaction.query_row(
            "SELECT s.spec_json,s.spec_hash,s.spec_revision,s.policy_revision,t.generation FROM task_specs s JOIN tasks t USING(task_id)
             WHERE s.task_id=?1 AND t.recorded_state='running'", [task_id],
             |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))
            .optional()?.ok_or(StoreError::Conflict)?;
        if digest(&spec_json) != spec_hash {
            return Err(StoreError::InvalidSpec);
        }
        let spec: TaskSpec =
            serde_json::from_str(&spec_json).map_err(|_| StoreError::InvalidSpec)?;
        let (canonical, path, allowed) = match tool_name {
            "read_input" => {
                let parsed: ReadArgs =
                    serde_json::from_str(args).map_err(|_| StoreError::InvalidToolCall)?;
                (
                    serde_json::to_string(&parsed),
                    spec.task.input,
                    spec.task.allow_read,
                )
            }
            "write_output" => {
                let parsed: WriteArgs =
                    serde_json::from_str(args).map_err(|_| StoreError::InvalidToolCall)?;
                if parsed.content.len() as u64 > FILE_LIMIT {
                    return Err(StoreError::InvalidToolCall);
                }
                (
                    serde_json::to_string(&parsed),
                    spec.task.output,
                    spec.task.allow_write,
                )
            }
            _ => return Err(StoreError::InvalidToolCall),
        };
        let args_json = canonical.map_err(|_| StoreError::InvalidSpec)?;
        let scope = path
            .to_str()
            .filter(|_| path.is_absolute())
            .ok_or(StoreError::InvalidSpec)?;
        let calls: i64 = transaction.query_row(
            "SELECT count(*) FROM tool_intents WHERE task_id=?1",
            [task_id],
            |r| r.get(0),
        )?;
        if calls >= TOOL_CALL_LIMIT {
            return Err(StoreError::InvalidSpec);
        }
        let status = if !allowed {
            "denied"
        } else if spec.approval_mode == ApprovalMode::Manual {
            "pending"
        } else {
            "approved"
        };
        let intent_id = Uuid::new_v4().to_string();
        let approval_id = Uuid::new_v4().to_string();
        let args_hash = digest(&args_json);
        transaction.execute("INSERT INTO tool_intents(intent_id,task_id,generation,call_id,tool_name,tool_version,args_json,args_hash,scope_path,status,step_id) VALUES (?1,?2,?3,?4,?5,1,?6,?7,?8,?9,?10)",
            params![intent_id,task_id,generation,call_id,tool_name,args_json,args_hash,scope,if allowed { "prepared" } else { "denied" },step_id])?;
        transaction.execute("INSERT INTO approvals(approval_id,intent_id,spec_revision,policy_revision,spec_hash,args_hash,tool_version,status,decision_source,expires_at,uses_remaining,decided_at)
            VALUES (?1,?2,?3,?4,?5,?6,1,?7,?8,unixepoch()+?9,1,CASE WHEN ?7='pending' THEN NULL ELSE strftime('%Y-%m-%dT%H:%M:%fZ','now') END)",
            params![approval_id,intent_id,spec_revision,policy_revision,spec_hash,args_hash,status,if status == "pending" { "pending" } else { "task_policy" },APPROVAL_TTL_SECONDS])?;
        append(
            &transaction,
            task_id,
            if status == "pending" {
                "approval_requested"
            } else if allowed {
                "approval_granted"
            } else {
                "approval_denied"
            },
            Some(if status == "pending" {
                "waiting_approval"
            } else {
                "running"
            }),
            &approval_id,
        )?;
        transaction.commit()?;
        Ok(PreparedIntent {
            intent_id,
            approval_id,
            task_id: task_id.into(),
            call_id: call_id.into(),
            args_hash,
            spec_revision,
        })
    }

    pub fn approvals(&self, task_id: &str) -> Result<Vec<ApprovalRecord>, StoreError> {
        self.status(task_id)?;
        let mut query = self.connection.prepare(
            "SELECT a.approval_id,i.task_id,a.spec_revision,a.policy_revision,i.tool_name,i.tool_version,i.scope_path,i.args_json,a.args_hash,a.status,a.uses_remaining,a.expires_at,a.decision_source,i.intent_id,i.status,i.step_id
             FROM approvals a JOIN tool_intents i USING(intent_id) WHERE i.task_id=?1 ORDER BY i.rowid LIMIT 100")?;
        let rows = query.query_map([task_id], |r| {
            Ok(ApprovalRecord {
                intent_id: r.get(13)?,
                intent_status: r.get(14)?,
                step_id: r.get(15)?,
                approval_id: r.get(0)?,
                task_id: r.get(1)?,
                spec_revision: r.get(2)?,
                policy_revision: r.get(3)?,
                tool_name: r.get(4)?,
                tool_version: r.get(5)?,
                scope_path: r.get(6)?,
                args_json: r.get(7)?,
                args_hash: r.get(8)?,
                status: r.get(9)?,
                uses_remaining: r.get(10)?,
                expires_at: r.get(11)?,
                decision_source: r.get(12)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn decide(
        &mut self,
        approval_id: &str,
        spec_revision: i64,
        approve: bool,
    ) -> Result<(), StoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let task_id: String = transaction.query_row(
            "SELECT i.task_id FROM approvals a JOIN tool_intents i USING(intent_id) JOIN tasks t USING(task_id) JOIN task_specs s USING(task_id)
             WHERE a.approval_id=?1 AND a.spec_revision=?2 AND a.spec_revision=s.spec_revision AND a.policy_revision=s.policy_revision AND a.spec_hash=s.spec_hash
             AND a.uses_remaining=1 AND a.expires_at>unixepoch() AND (a.status='pending' OR (?3=0 AND a.status='approved'))
             AND i.status='prepared' AND i.generation=t.generation AND t.recorded_state IN ('running','waiting_approval')",
            params![approval_id,spec_revision,approve], |r| r.get(0)).optional()?.ok_or(StoreError::Conflict)?;
        transaction.execute("UPDATE approvals SET status=?2,decision_source='local_cli',decided_at=strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE approval_id=?1", params![approval_id, if approve { "approved" } else { "denied" }])?;
        append(
            &transaction,
            &task_id,
            if approve {
                "approval_granted"
            } else {
                "approval_denied"
            },
            None,
            approval_id,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn claim(&mut self, prepared: &PreparedIntent) -> Result<Claim, StoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let value: Option<(String,i64,bool)> = transaction.query_row(
            "SELECT a.status,a.uses_remaining,a.expires_at>unixepoch() FROM approvals a JOIN tool_intents i USING(intent_id) JOIN tasks t USING(task_id) JOIN task_specs s USING(task_id)
             WHERE a.approval_id=?1 AND i.task_id=?2 AND i.call_id=?3 AND a.args_hash=?4 AND i.args_hash=a.args_hash AND a.spec_revision=?5
             AND a.spec_revision=s.spec_revision AND a.policy_revision=s.policy_revision AND a.spec_hash=s.spec_hash AND a.tool_version=i.tool_version
             AND i.generation=t.generation AND i.intent_id=?6 AND t.recorded_state IN ('running','waiting_approval')",
            params![prepared.approval_id,prepared.task_id,prepared.call_id,prepared.args_hash,prepared.spec_revision,prepared.intent_id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
        let (status, uses, valid) = value.ok_or(StoreError::Conflict)?;
        if uses != 1 || status == "denied" || status == "revoked" || status == "expired" {
            return Ok(Claim::Denied);
        }
        if !valid {
            transaction.execute(
                "UPDATE approvals SET status='expired' WHERE approval_id=?1",
                [&prepared.approval_id],
            )?;
            append(
                &transaction,
                &prepared.task_id,
                "approval_expired",
                None,
                &prepared.approval_id,
            )?;
            transaction.commit()?;
            return Ok(Claim::Denied);
        }
        if status == "pending" {
            return Ok(Claim::Pending);
        }
        if status != "approved" {
            return Err(StoreError::Conflict);
        }
        transaction.execute(
            "UPDATE approvals SET uses_remaining=0 WHERE approval_id=?1",
            [&prepared.approval_id],
        )?;
        let changed = transaction.execute(
            "UPDATE tool_intents SET status='dispatched' WHERE intent_id=?1 AND status='prepared'",
            [&prepared.intent_id],
        )?;
        if changed != 1 {
            return Err(StoreError::Conflict);
        }
        append(
            &transaction,
            &prepared.task_id,
            "tool_dispatched",
            Some("running"),
            &prepared.approval_id,
        )?;
        transaction.commit()?;
        Ok(Claim::Granted)
    }

    pub fn receipt(
        &mut self,
        task_id: &str,
        step_id: i64,
        call_id: &str,
        succeeded: bool,
    ) -> Result<(), StoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let intent_id: String = transaction.query_row("UPDATE tool_intents SET status=?3 WHERE task_id=?1 AND call_id=?2 AND step_id=?4 AND generation=(SELECT generation FROM tasks WHERE task_id=?1) AND status='dispatched' RETURNING intent_id",
            params![task_id,call_id,if succeeded { "succeeded" } else { "failed" },step_id], |r| r.get(0)).optional()?.ok_or(StoreError::Conflict)?;
        append(
            &transaction,
            task_id,
            if succeeded {
                "tool_succeeded"
            } else {
                "tool_failed"
            },
            None,
            &intent_id,
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn record_rejection(&mut self, task_id: &str, dispatch_id: &str) -> Result<(), StoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        append(&transaction, task_id, "tool_rejected", None, dispatch_id)?;
        transaction.commit()?;
        Ok(())
    }
}
