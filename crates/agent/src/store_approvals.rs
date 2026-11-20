use crate::prelude::*;
use crate::task::{FILE_LIMIT, FileTask, READ_INPUT_TOOL, ReadArgs, WRITE_OUTPUT_TOOL, WriteArgs};
use serde::Deserialize;
use std::path::PathBuf;

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
        if call_id.is_empty() || call_id.len() > CALL_ID_LIMIT || args.len() > MAX_ARGS_BYTES {
            return Err(StoreError::InvalidToolCall);
        }
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (spec_json, spec_hash, spec_revision, policy_revision, generation): (String,String,i64,i64,i64) = transaction.query_row(
            "SELECT s.spec_json,s.spec_hash,s.spec_revision,s.policy_revision,t.generation FROM task_specs s JOIN tasks t USING(task_id)
             WHERE s.task_id=?1 AND t.recorded_state=?2", params![task_id, TaskState::Running],
             |r| Ok((r.get("spec_json")?,r.get("spec_hash")?,r.get("spec_revision")?,r.get("policy_revision")?,r.get("generation")?)))
            .optional()?.ok_or(StoreError::Conflict)?;
        if digest(&spec_json) != spec_hash {
            return Err(StoreError::InvalidSpec);
        }
        let spec: TaskSpec =
            serde_json::from_str(&spec_json).map_err(|_| StoreError::InvalidSpec)?;
        let (canonical, path, allowed) = match tool_name {
            READ_INPUT_TOOL => {
                let parsed: ReadArgs =
                    serde_json::from_str(args).map_err(|_| StoreError::InvalidToolCall)?;
                (
                    serde_json::to_string(&parsed),
                    spec.task.input,
                    spec.task.allow_read,
                )
            }
            WRITE_OUTPUT_TOOL => {
                let parsed: WriteArgs = serde_json::from_str(args)
                    .ok()
                    .filter(|parsed: &WriteArgs| parsed.content.len() as u64 <= FILE_LIMIT)
                    .ok_or(StoreError::InvalidToolCall)?;
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
            ApprovalStatus::Denied
        } else if spec.approval_mode == ApprovalMode::Manual {
            ApprovalStatus::Pending
        } else {
            ApprovalStatus::Approved
        };
        let pending = status == ApprovalStatus::Pending;
        let intent_id = Uuid::new_v4().to_string();
        let approval_id = Uuid::new_v4().to_string();
        let args_hash = digest(&args_json);
        let intent_status = if allowed {
            IntentStatus::Prepared
        } else {
            IntentStatus::Denied
        };
        let source = if pending {
            DecisionSource::Pending
        } else {
            DecisionSource::TaskPolicy
        };
        transaction.execute("INSERT INTO tool_intents(intent_id,task_id,generation,call_id,tool_name,tool_version,args_json,args_hash,scope_path,status,step_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![intent_id,task_id,generation,call_id,tool_name,TOOL_VERSION,args_json,args_hash,scope,intent_status,step_id])?;
        transaction.execute(&format!("INSERT INTO approvals(approval_id,intent_id,spec_revision,policy_revision,spec_hash,args_hash,tool_version,status,decision_source,expires_at,uses_remaining,decided_at)
            VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,unixepoch()+?10,?11,CASE WHEN ?8=?12 THEN NULL ELSE {TIMESTAMP_NOW} END)"),
            params![approval_id,intent_id,spec_revision,policy_revision,spec_hash,args_hash,TOOL_VERSION,status,source,APPROVAL_TTL_SECONDS,USES_AVAILABLE,ApprovalStatus::Pending])?;
        let kind = if pending {
            EventKind::ApprovalRequested
        } else if allowed {
            EventKind::ApprovalGranted
        } else {
            EventKind::ApprovalDenied
        };
        let state = if pending {
            TaskState::WaitingApproval
        } else {
            TaskState::Running
        };
        append(&transaction, task_id, kind, Some(state), &approval_id)?;
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
            "SELECT a.approval_id,i.task_id,a.spec_revision,a.policy_revision,i.tool_name,i.tool_version,i.scope_path,i.args_json,a.args_hash,a.status AS approval_status,a.uses_remaining,a.expires_at,a.decision_source,i.intent_id,i.status AS intent_status,i.step_id
             FROM approvals a JOIN tool_intents i USING(intent_id) WHERE i.task_id=?1 ORDER BY i.rowid LIMIT ?2")?;
        let rows = query.query_map(params![task_id, APPROVAL_LIST_LIMIT], |r| {
            Ok(ApprovalRecord {
                intent_id: r.get("intent_id")?,
                intent_status: r.get("intent_status")?,
                step_id: r.get("step_id")?,
                approval_id: r.get("approval_id")?,
                task_id: r.get("task_id")?,
                spec_revision: r.get("spec_revision")?,
                policy_revision: r.get("policy_revision")?,
                tool_name: r.get("tool_name")?,
                tool_version: r.get("tool_version")?,
                scope_path: r.get("scope_path")?,
                args_json: r.get("args_json")?,
                args_hash: r.get("args_hash")?,
                status: r.get("approval_status")?,
                uses_remaining: r.get("uses_remaining")?,
                expires_at: r.get("expires_at")?,
                decision_source: r.get("decision_source")?,
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
             AND a.uses_remaining=?4 AND a.expires_at>unixepoch() AND (a.status=?5 OR (NOT ?3 AND a.status=?6))
             AND i.status=?7 AND i.generation=t.generation AND t.recorded_state IN (?8,?9)",
            params![approval_id,spec_revision,approve,USES_AVAILABLE,ApprovalStatus::Pending,ApprovalStatus::Approved,IntentStatus::Prepared,TaskState::Running,TaskState::WaitingApproval], |r| r.get(0)).optional()?.ok_or(StoreError::Conflict)?;
        transaction.execute(&format!("UPDATE approvals SET status=?2,decision_source=?3,decided_at={TIMESTAMP_NOW} WHERE approval_id=?1"), params![approval_id, if approve { ApprovalStatus::Approved } else { ApprovalStatus::Denied }, DecisionSource::LocalCli])?;
        append(
            &transaction,
            &task_id,
            if approve {
                EventKind::ApprovalGranted
            } else {
                EventKind::ApprovalDenied
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
            "SELECT a.status AS approval_status,a.uses_remaining,a.expires_at>unixepoch() AS unexpired FROM approvals a JOIN tool_intents i USING(intent_id) JOIN tasks t USING(task_id) JOIN task_specs s USING(task_id)
             WHERE a.approval_id=?1 AND i.task_id=?2 AND i.call_id=?3 AND a.args_hash=?4 AND i.args_hash=a.args_hash AND a.spec_revision=?5
             AND a.spec_revision=s.spec_revision AND a.policy_revision=s.policy_revision AND a.spec_hash=s.spec_hash AND a.tool_version=i.tool_version
             AND i.generation=t.generation AND i.intent_id=?6 AND t.recorded_state IN (?7,?8)",
            params![prepared.approval_id,prepared.task_id,prepared.call_id,prepared.args_hash,prepared.spec_revision,prepared.intent_id,TaskState::Running,TaskState::WaitingApproval], |r| Ok((r.get("approval_status")?,r.get("uses_remaining")?,r.get("unexpired")?))).optional()?;
        let (status, uses, valid) = value.ok_or(StoreError::Conflict)?;
        let closed = [
            ApprovalStatus::Denied,
            ApprovalStatus::Revoked,
            ApprovalStatus::Expired,
        ]
        .iter()
        .any(|closed| closed.as_str() == status);
        if uses != USES_AVAILABLE || closed {
            return Ok(Claim::Denied);
        }
        if !valid {
            transaction.execute(
                "UPDATE approvals SET status=?2 WHERE approval_id=?1",
                params![prepared.approval_id, ApprovalStatus::Expired],
            )?;
            append(
                &transaction,
                &prepared.task_id,
                EventKind::ApprovalExpired,
                None,
                &prepared.approval_id,
            )?;
            transaction.commit()?;
            return Ok(Claim::Denied);
        }
        if status == ApprovalStatus::Pending.as_str() {
            return Ok(Claim::Pending);
        }
        if status != ApprovalStatus::Approved.as_str() {
            return Err(StoreError::Conflict);
        }
        transaction.execute(
            "UPDATE approvals SET uses_remaining=?2 WHERE approval_id=?1",
            params![prepared.approval_id, USES_SPENT],
        )?;
        let changed = transaction.execute(
            "UPDATE tool_intents SET status=?2 WHERE intent_id=?1 AND status=?3",
            params![
                prepared.intent_id,
                IntentStatus::Dispatched,
                IntentStatus::Prepared
            ],
        )?;
        if changed != EXPECTED_ROWS_CHANGED {
            return Err(StoreError::Conflict);
        }
        append(
            &transaction,
            &prepared.task_id,
            EventKind::ToolDispatched,
            Some(TaskState::Running),
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
        let intent_id: String = transaction.query_row("UPDATE tool_intents SET status=?3 WHERE task_id=?1 AND call_id=?2 AND step_id=?4 AND generation=(SELECT generation FROM tasks WHERE task_id=?1) AND status=?5 RETURNING intent_id",
            params![task_id,call_id,if succeeded { IntentStatus::Succeeded } else { IntentStatus::Failed },step_id,IntentStatus::Dispatched], |r| r.get(0)).optional()?.ok_or(StoreError::Conflict)?;
        append(
            &transaction,
            task_id,
            if succeeded {
                EventKind::ToolSucceeded
            } else {
                EventKind::ToolFailed
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
        append(
            &transaction,
            task_id,
            EventKind::ToolRejected,
            None,
            dispatch_id,
        )?;
        transaction.commit()?;
        Ok(())
    }
}
