use crate::task::{ArtifactState, TaskError, TaskReport};
use recovery_call::RecoveryCall;
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::Serialize;
use std::fs::File;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;
mod append;
mod approvals;
mod close_intents;
mod digest;
mod recovery_call;
mod recovery_session;
mod result_snapshot;
pub use approvals::{
    APPROVAL_TTL_SECONDS, ApprovalMode, ApprovalRecord, Claim, PreparedIntent, TOOL_CALL_LIMIT,
    TaskSpec,
};
pub use result_snapshot::{ResultSnapshot, SavedToolCall};

const APPLICATION_ID: i64 = 0x41504754;
pub const SCHEMA_VERSION: i64 = 8;
pub const EVENT_PAGE_SIZE: i64 = 100;
pub const INSTRUCTION_ID_LIMIT: usize = 128;
pub const INSTRUCTION_LIMIT: usize = 8192;
pub const INSTRUCTION_PENDING_LIMIT: i64 = 16;

#[derive(Clone, Copy, PartialEq)]
pub enum StoreAccess {
    Read,
    Write,
    Create,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("無法保存或讀取任務紀錄")]
    Database(#[from] rusqlite::Error),
    #[error("任務資料庫版本或設定不相容")]
    Incompatible,
    #[error("找不到任務")]
    NotFound,
    #[error("任務紀錄已變動，不可用舊狀態保存結果或重複執行")]
    Conflict,
    #[error("對話或工具結果尚未確認，不能自動接續；請核對紀錄與成果")]
    Unsettled,
    #[error(transparent)]
    Session(#[from] agent_harness::SessionError),
    #[error("查詢位置無效；事件位置須非負整數，任務位置須沿用清單回傳值")]
    InvalidCursor,
    #[error("任務規格不符合限制，不能繼續執行")]
    InvalidSpec,
    #[error("工具名稱或參數不符合限制")]
    InvalidToolCall,
    #[error("補充指令的訊息編號或內容不符合限制")]
    InvalidInstruction,
    #[error("待處理的補充指令已滿，請等模型讀取後再補")]
    InstructionBacklog,
    #[error("無法核對或停止任務程序：{0}")]
    Process(#[from] std::io::Error),
}

#[derive(Debug, Serialize)]
pub struct TaskRecord {
    pub task_id: String,
    pub schema_version: i64,
    pub generation: i64,
    pub recorded_state: String,
    pub artifact_created: Option<bool>,
    pub last_seq: i64,
    pub updated_at: String,
    pub pending_instructions: i64,
}

#[derive(Debug, Serialize)]
pub struct InstructionReceipt {
    pub seq: i64,
    pub duplicate: bool,
}

#[derive(Debug, Serialize)]
pub struct TaskEvent {
    pub event_id: String,
    pub task_id: String,
    pub schema_version: i64,
    pub generation: i64,
    pub seq: i64,
    pub event_type: String,
    pub created_at: String,
    pub related_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TaskSummary {
    pub task_id: String,
    pub recorded_state: String,
    pub workspace: Option<PathBuf>,
}

impl TaskSummary {
    fn from_saved(
        task_id: String,
        recorded_state: String,
        json: Option<String>,
        hash: Option<String>,
    ) -> Result<Self, StoreError> {
        if json.is_none() && hash.is_none() {
            return Ok(Self {
                task_id,
                recorded_state,
                workspace: None,
            });
        }
        let json = json.ok_or(StoreError::InvalidSpec)?;
        let hash = hash.ok_or(StoreError::InvalidSpec)?;
        if json.len() > 131072 || digest::digest(&json) != hash {
            return Err(StoreError::InvalidSpec);
        }
        let spec: TaskSpec = serde_json::from_str(&json).map_err(|_| StoreError::InvalidSpec)?;
        Ok(Self {
            task_id,
            recorded_state,
            workspace: Some(spec.workspace),
        })
    }
}

#[derive(Debug, Serialize)]
pub struct TaskPage {
    pub schema_version: u32,
    pub tasks: Vec<TaskSummary>,
    pub next_cursor: Option<String>,
}

pub struct TaskStore {
    pub(crate) connection: Connection,
    schema_version: i64,
}

impl TaskStore {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::connect(path, StoreAccess::Create)
    }

    pub fn read(path: &Path) -> Result<Self, StoreError> {
        Self::connect(path, StoreAccess::Read)
    }

    pub fn write(path: &Path) -> Result<Self, StoreError> {
        Self::connect(path, StoreAccess::Write)
    }

    fn connect(path: &Path, access: StoreAccess) -> Result<Self, StoreError> {
        let create = access == StoreAccess::Create;
        if rusqlite::version_number() < 3_051_003 {
            return Err(StoreError::Incompatible);
        }
        let flags = if create {
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
        } else if access == StoreAccess::Write {
            OpenFlags::SQLITE_OPEN_READ_WRITE
        } else {
            OpenFlags::SQLITE_OPEN_READ_ONLY
        };
        let mut connection = Connection::open_with_flags(
            path,
            flags | OpenFlags::SQLITE_OPEN_NO_MUTEX | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?;
        connection.busy_timeout(Duration::from_millis(500))?;
        let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let application: i64 =
            connection.pragma_query_value(None, "application_id", |r| r.get(0))?;
        if !matches!(
            (application, version),
            (0, 0) | (APPLICATION_ID, 1..=SCHEMA_VERSION)
        ) {
            return Err(StoreError::Incompatible);
        }
        if version == 0 && !create {
            return Err(StoreError::Incompatible);
        }
        if access == StoreAccess::Write && version != SCHEMA_VERSION {
            return Err(StoreError::Incompatible);
        }
        if create && version < SCHEMA_VERSION {
            Self::migrate(&mut connection)?;
        }
        let journal: String = if create {
            connection.pragma_update_and_check(None, "journal_mode", "wal", |r| r.get(0))?
        } else {
            connection.pragma_query_value(None, "journal_mode", |r| r.get(0))?
        };
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.pragma_update(None, "foreign_keys", true)?;
        connection.pragma_update(None, "trusted_schema", false)?;
        let synchronous: i64 = connection.pragma_query_value(None, "synchronous", |r| r.get(0))?;
        let foreign_keys: i64 =
            connection.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
        if (create && journal != "wal") || synchronous != 2 || foreign_keys != 1 {
            return Err(StoreError::Incompatible);
        }
        let schema_version = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
        Ok(Self {
            connection,
            schema_version,
        })
    }

    fn migrate(connection: &mut Connection) -> Result<(), StoreError> {
        connection.pragma_update(None, "foreign_keys", false)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut version = Self::stored_version(&transaction)?;
        if version == 1 {
            transaction.execute_batch(include_str!("store_v2.sql"))?;
            version = 2;
        }
        if version == 2 {
            transaction.execute_batch(include_str!("store_v3.sql"))?;
            version = 3;
        }
        if version == 3 {
            transaction.execute_batch(include_str!("store_v4.sql"))?;
        }
        if version < 6 {
            transaction.execute_batch(include_str!("store_v6.sql"))?;
        }
        if version < 7 {
            transaction.execute_batch(include_str!("store_v7.sql"))?;
        }
        if version < 8 {
            transaction.execute_batch(include_str!("store_v8.sql"))?;
        }
        if transaction
            .query_row("PRAGMA foreign_key_check", [], |_| Ok(()))
            .optional()?
            .is_some()
        {
            return Err(StoreError::Incompatible);
        }
        transaction.pragma_update(None, "user_version", SCHEMA_VERSION)?;
        transaction.commit()?;
        Ok(())
    }

    fn stored_version(transaction: &Transaction<'_>) -> Result<i64, StoreError> {
        let version: i64 = transaction.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let application: i64 =
            transaction.pragma_query_value(None, "application_id", |r| r.get(0))?;
        if (application, version) == (0, 0) {
            return Self::install_first_version(transaction);
        }
        if application != APPLICATION_ID || !(1..=SCHEMA_VERSION).contains(&version) {
            return Err(StoreError::Incompatible);
        }
        Ok(version)
    }

    fn install_first_version(transaction: &Transaction<'_>) -> Result<i64, StoreError> {
        let tables: i64 = transaction.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get(0),
        )?;
        if tables != 0 {
            return Err(StoreError::Incompatible);
        }
        transaction.execute_batch(include_str!("store_v1.sql"))?;
        transaction.pragma_update(None, "application_id", APPLICATION_ID)?;
        Ok(1)
    }

    fn missing_cause(
        transaction: &Transaction<'_>,
        task_id: &str,
    ) -> Result<StoreError, StoreError> {
        let exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE task_id=?1)",
            [task_id],
            |r| r.get(0),
        )?;
        if exists {
            return Ok(StoreError::Conflict);
        }
        Ok(StoreError::NotFound)
    }

    pub fn start(&mut self) -> Result<(String, i64), StoreError> {
        self.create_task(None)
    }

    pub fn start_spec(&mut self, spec: &TaskSpec) -> Result<(String, i64), StoreError> {
        self.create_task(Some(spec))
    }

    fn create_task(&mut self, spec: Option<&TaskSpec>) -> Result<(String, i64), StoreError> {
        let spec_json = spec
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| StoreError::InvalidSpec)?;
        if spec_json.as_ref().is_some_and(|value| value.len() > 131072) {
            return Err(StoreError::InvalidSpec);
        }
        let task_id = Uuid::new_v4().to_string();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (generation, seq) = transaction.query_row(
            "INSERT INTO tasks (task_id, recorded_state, last_seq) VALUES (?1, 'running', 1) RETURNING generation, last_seq",
            [&task_id],
            |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)),
        )?;
        transaction.execute(
            "INSERT INTO events (event_id, task_id, seq, event_type) VALUES (?1, ?2, ?3, 'started')",
            params![Uuid::new_v4().to_string(), task_id, seq],
        )?;
        if let Some(spec_json) = spec_json {
            transaction.execute(
                "INSERT INTO task_specs(task_id,spec_json,spec_hash) VALUES (?1,?2,?3)",
                params![task_id, spec_json, digest::digest(&spec_json)],
            )?;
        }
        transaction.commit()?;
        Ok((task_id, generation))
    }

    pub fn finish(
        &mut self,
        task_id: &str,
        expected_generation: i64,
        result: &Result<TaskReport, TaskError>,
    ) -> Result<(), StoreError> {
        let state = match result {
            Ok(report) => match report.artifact {
                ArtifactState::Verified => "succeeded",
                ArtifactState::AwaitingReview => "needs_review",
            },
            Err(TaskError::Cancelled { .. }) => "cancelled",
            Err(TaskError::Timeout | TaskError::Harness(_) | TaskError::State) => "outcome_unknown",
            Err(_) => "failed",
        };
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let unresolved: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM tool_intents WHERE task_id=?1 AND status='dispatched')",
            [task_id],
            |r| r.get(0),
        )?;
        let requested: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE task_id=?1 AND generation=?2 AND recorded_state='cancel_requested')",
            params![task_id, expected_generation], |row| row.get(0),
        )?;
        let stopped_before_dispatch = matches!(
            result,
            Err(TaskError::Harness(agent_harness::RunError::Stopped(_)))
        );
        let state = match (unresolved, requested, state) {
            (true, _, _) => "outcome_unknown",
            (_, true, _) if stopped_before_dispatch => "cancelled",
            (_, _, "outcome_unknown") => "outcome_unknown",
            (_, true, _) => "cancelled",
            _ => state,
        };
        let artifact_created = match result {
            Ok(_) | Err(TaskError::IncompleteArtifact | TaskError::Verification(_)) => Some(true),
            Err(TaskError::Cancelled { artifact_created }) => Some(*artifact_created),
            Err(
                TaskError::InvalidTask
                | TaskError::InvalidInput
                | TaskError::Input(_)
                | TaskError::MissingArtifact,
            ) => Some(false),
            Err(_) => None,
        };
        let seq: Option<i64> = transaction.query_row(
            "UPDATE tasks SET recorded_state = ?1, last_seq = last_seq + 1, artifact_created = ?4,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
             WHERE task_id = ?2 AND recorded_state IN ('running','waiting_approval','cancel_requested') AND generation = ?3 RETURNING last_seq",
            params![state, task_id, expected_generation, artifact_created], |r| r.get(0),
        ).optional()?;
        let Some(seq) = seq else {
            return Err(Self::missing_cause(&transaction, task_id)?);
        };
        transaction.execute(
            "INSERT INTO events (event_id, task_id, seq, event_type, generation) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![Uuid::new_v4().to_string(), task_id, seq, state, expected_generation],
        )?;
        close_intents::close_intents(&transaction, task_id)?;
        transaction.execute(
            "UPDATE tool_intents SET status='cancelled' WHERE task_id=?1 AND status='prepared'",
            [task_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn list(&self, after: Option<&str>) -> Result<TaskPage, StoreError> {
        if after.is_some_and(|cursor| Uuid::parse_str(cursor).is_err()) {
            return Err(StoreError::InvalidCursor);
        }
        let specs = if self.schema_version == 1 {
            "NULL, NULL FROM tasks t"
        } else {
            "substr(s.spec_json,1,131073), s.spec_hash FROM tasks t LEFT JOIN task_specs s USING(task_id)"
        };
        let mut query = self.connection.prepare(&format!(
            "SELECT t.task_id,t.recorded_state,{specs} WHERE t.task_id > ?1 ORDER BY t.task_id LIMIT 101"
        ))?;
        let mut rows = query.query_map([after.unwrap_or("")], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let mut tasks = Vec::new();
        for row in rows.by_ref().take(100) {
            let (task_id, recorded_state, json, hash) = row?;
            tasks.push(TaskSummary::from_saved(
                task_id,
                recorded_state,
                json,
                hash,
            )?);
        }
        let next_cursor = if rows.next().transpose()?.is_some() {
            tasks.last().map(|task| task.task_id.clone())
        } else {
            None
        };
        Ok(TaskPage {
            schema_version: 1,
            tasks,
            next_cursor,
        })
    }

    pub fn status(&self, task_id: &str) -> Result<TaskRecord, StoreError> {
        let pending = if self.schema_version >= 8 {
            "(SELECT count(*) FROM instructions i WHERE i.task_id = tasks.task_id AND i.status = 'received')"
        } else {
            "0"
        };
        self.connection
            .query_row(
                &format!(
                    "SELECT task_id, schema_version, generation, recorded_state, last_seq, updated_at, artifact_created, {pending}
             FROM tasks WHERE task_id = ?1"
                ),
                [task_id],
                |r| {
                    Ok(TaskRecord {
                        task_id: r.get(0)?,
                        schema_version: r.get(1)?,
                        generation: r.get(2)?,
                        recorded_state: r.get(3)?,
                        last_seq: r.get(4)?,
                        updated_at: r.get(5)?,
                        artifact_created: r.get(6)?,
                        pending_instructions: r.get(7)?,
                    })
                },
            )
            .optional()?
            .ok_or(StoreError::NotFound)
    }

    pub fn events(&self, task_id: &str, after_seq: i64) -> Result<Vec<TaskEvent>, StoreError> {
        if after_seq < 0 {
            return Err(StoreError::InvalidCursor);
        }
        self.status(task_id)?;
        let related = if self.schema_version == 1 {
            "NULL"
        } else {
            "related_id"
        };
        let mut query = self.connection.prepare(&format!(
            "SELECT event_id, task_id, schema_version, generation, seq, event_type, created_at, {related}
             FROM events WHERE task_id = ?1 AND seq > ?2 ORDER BY seq LIMIT ?3"
        ))?;
        let rows = query.query_map(params![task_id, after_seq, EVENT_PAGE_SIZE], |r| {
            Ok(TaskEvent {
                event_id: r.get(0)?,
                task_id: r.get(1)?,
                schema_version: r.get(2)?,
                generation: r.get(3)?,
                seq: r.get(4)?,
                event_type: r.get(5)?,
                created_at: r.get(6)?,
                related_id: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

impl TaskStore {
    pub(crate) fn resume_task(
        &mut self,
        task_id: &str,
        expected_generation: i64,
        expected_spec: &TaskSpec,
    ) -> Result<(i64, Option<File>), StoreError> {
        let snapshot = self.result_snapshot(task_id)?;
        if snapshot.record.generation != expected_generation
            || snapshot.spec.as_ref() != Some(expected_spec)
            || !matches!(
                snapshot.record.recorded_state.as_str(),
                "running"
                    | "waiting_approval"
                    | "cancel_requested"
                    | "failed"
                    | "cancelled"
                    | "outcome_unknown"
            )
        {
            return Err(StoreError::Conflict);
        }
        let calls = self.recovery_calls(task_id, expected_generation)?;
        let saved = recovery_session::recovery_session(&snapshot, &calls)?;
        let artifact = crate::result::TaskResult::restore_file(&snapshot)
            .map_err(|_| StoreError::Unsettled)?;
        let generation = expected_generation
            .checked_add(1)
            .ok_or(StoreError::Conflict)?;
        let json = serde_json::to_string(&saved).map_err(|_| StoreError::InvalidSpec)?;
        if json.len() > result_snapshot::SESSION_LIMIT {
            return Err(StoreError::InvalidSpec);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE task_id=?1 AND generation=?2 AND last_seq=?3)",
            params![task_id, expected_generation, snapshot.record.last_seq],
            |row| row.get(0),
        )?;
        if !current {
            return Err(StoreError::Conflict);
        }
        close_intents::close_intents(&tx, task_id)?;
        tx.execute(
            "UPDATE tool_intents SET status='cancelled' WHERE task_id=?1 AND status='prepared'",
            [task_id],
        )?;
        tx.execute(
            "INSERT INTO task_runs(task_id,generation,recorded_state,last_seq,artifact_created,session_revision,session_json)
             SELECT t.task_id,t.generation,t.recorded_state,t.last_seq,t.artifact_created,s.revision,s.session_json FROM tasks t LEFT JOIN sessions s USING(task_id) WHERE t.task_id=?1",
            [task_id],
        )?;
        tx.execute("UPDATE tasks SET generation=?2,recorded_state='running',artifact_created=NULL WHERE task_id=?1", params![task_id,generation])?;
        tx.execute(
            "INSERT INTO sessions(task_id,generation,revision,session_json) VALUES (?1,?2,1,?3)
             ON CONFLICT(task_id) DO UPDATE SET generation=excluded.generation,revision=excluded.revision,session_json=excluded.session_json",
            params![task_id,generation,json],
        )?;
        append::append(
            &tx,
            task_id,
            "started",
            None,
            &expected_generation.to_string(),
        )?;
        tx.commit()?;
        Ok((generation, artifact))
    }

    fn recovery_calls(
        &self,
        task_id: &str,
        generation: i64,
    ) -> Result<Vec<RecoveryCall>, StoreError> {
        let invalid: bool = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM tool_intents i LEFT JOIN approvals a USING(intent_id) JOIN task_specs s USING(task_id)
             WHERE i.task_id=?1 AND (i.generation>?2 OR a.approval_id IS NULL OR a.spec_revision!=s.spec_revision OR a.policy_revision!=s.policy_revision
             OR a.spec_hash!=s.spec_hash OR a.args_hash!=i.args_hash OR a.tool_version!=i.tool_version
             OR (i.status IN ('succeeded','failed') AND (a.status!='approved' OR a.uses_remaining!=0))))",
            params![task_id,generation], |row| row.get(0),
        )?;
        if invalid {
            return Err(StoreError::Unsettled);
        }
        let mut query = self.connection.prepare(
            "SELECT call_id,tool_name,substr(args_json,1,?2),args_hash,scope_path,status FROM tool_intents WHERE task_id=?1 ORDER BY generation,step_id,intent_id LIMIT ?3",
        )?;
        let rows = query.query_map(
            params![
                task_id,
                6 * crate::task::FILE_LIMIT as i64 + 1025,
                TOOL_CALL_LIMIT + 1
            ],
            |row| {
                Ok(RecoveryCall {
                    call_id: row.get(0)?,
                    tool_name: row.get(1)?,
                    args_json: row.get(2)?,
                    args_hash: row.get(3)?,
                    scope_path: row.get(4)?,
                    status: row.get(5)?,
                })
            },
        )?;
        let calls = rows.collect::<Result<Vec<_>, _>>()?;
        if calls.len() as i64 > TOOL_CALL_LIMIT {
            return Err(StoreError::InvalidToolCall);
        }
        Ok(calls)
    }
}

impl TaskStore {
    pub fn request_cancel(
        &mut self,
        task_id: &str,
        expected_generation: i64,
    ) -> Result<(), StoreError> {
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (generation, state): (i64, String) = tx
            .query_row(
                "SELECT generation,recorded_state FROM tasks WHERE task_id=?1",
                [task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?
            .ok_or(StoreError::NotFound)?;
        if generation != expected_generation {
            return Err(StoreError::Conflict);
        }
        if matches!(state.as_str(), "cancel_requested" | "cancelled") {
            return Ok(());
        }
        append::append(
            &tx,
            task_id,
            "cancel_requested",
            Some("cancel_requested"),
            "",
        )?;
        tx.execute(
            "UPDATE tasks SET schema_version=3 WHERE task_id=?1 AND generation=?2",
            params![task_id, expected_generation],
        )?;
        tx.commit()?;
        Ok(())
    }

    pub fn receive_instruction(
        &mut self,
        task_id: &str,
        client_message_id: &str,
        content: &str,
    ) -> Result<InstructionReceipt, StoreError> {
        if client_message_id.is_empty()
            || client_message_id.len() > INSTRUCTION_ID_LIMIT
            || content.is_empty()
            || content.len() > INSTRUCTION_LIMIT
        {
            return Err(StoreError::InvalidInstruction);
        }
        let tx = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let state: String = tx
            .query_row(
                "SELECT recorded_state FROM tasks WHERE task_id=?1",
                [task_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::NotFound)?;
        let saved: Option<(i64, bool)> = tx
            .query_row(
                "SELECT seq,content=?3 FROM instructions WHERE task_id=?1 AND client_message_id=?2",
                params![task_id, client_message_id, content],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        if let Some((seq, same)) = saved {
            return same
                .then_some(InstructionReceipt {
                    seq,
                    duplicate: true,
                })
                .ok_or(StoreError::Conflict);
        }
        if !matches!(state.as_str(), "running" | "waiting_approval") {
            return Err(StoreError::Conflict);
        }
        let pending: i64 = tx.query_row(
            "SELECT count(*) FROM instructions WHERE task_id=?1 AND status='received'",
            [task_id],
            |row| row.get(0),
        )?;
        if pending >= INSTRUCTION_PENDING_LIMIT {
            return Err(StoreError::InstructionBacklog);
        }
        let seq = tx.query_row(
            "INSERT INTO instructions(task_id,client_message_id,seq,content) SELECT ?1,?2,coalesce(max(seq),0)+1,?3 FROM instructions WHERE task_id=?1 RETURNING seq",
            params![task_id, client_message_id, content],
            |row| row.get(0),
        )?;
        tx.commit()?;
        Ok(InstructionReceipt {
            seq,
            duplicate: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_list_checks_exact_spec_limit_and_rejects_valid_hash_with_bad_json()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(matches!(
            TaskSummary::from_saved("task".into(), "running".into(), Some("{}".into()), None),
            Err(StoreError::InvalidSpec)
        ));
        assert!(matches!(
            TaskSummary::from_saved("task".into(), "running".into(), None, Some("hash".into())),
            Err(StoreError::InvalidSpec)
        ));
        let mut connection = Connection::open_in_memory()?;
        TaskStore::migrate(&mut connection)?;
        let mut store = TaskStore {
            connection,
            schema_version: SCHEMA_VERSION,
        };
        let mut spec: TaskSpec = serde_json::from_value(serde_json::json!({
            "workspace": "/workspace", "endpoint": "http://127.0.0.1:9", "model": "fixture",
            "approval_mode": "manual", "task": {"prompt": "", "input": "input.txt", "output": "output.txt"}
        }))?;
        let overhead = serde_json::to_string(&spec)?.len();
        spec.task.prompt = "a".repeat(131072 - overhead);
        let task = store.start_spec(&spec)?.0;
        assert_eq!(
            store.list(None)?.tasks[0].workspace.as_ref(),
            Some(&spec.workspace)
        );
        spec.task.prompt.push('a');
        let oversized = serde_json::to_string(&spec)?;
        assert_eq!(oversized.len(), 131073);
        for json in [oversized.as_str(), "{}"] {
            store.connection.execute(
                "UPDATE task_specs SET spec_json=?1,spec_hash=?2 WHERE task_id=?3",
                params![json, digest::digest(json), task],
            )?;
            assert!(matches!(store.list(None), Err(StoreError::InvalidSpec)));
        }
        Ok(())
    }

    #[test]
    fn migration_transaction_rechecks_database_identity_and_supported_version()
    -> Result<(), Box<dyn std::error::Error>> {
        for (application, version, accepted) in [
            (APPLICATION_ID, SCHEMA_VERSION, true),
            (APPLICATION_ID + 1, SCHEMA_VERSION, false),
            (APPLICATION_ID, SCHEMA_VERSION + 1, false),
            (APPLICATION_ID + 1, SCHEMA_VERSION + 1, false),
        ] {
            let mut connection = Connection::open_in_memory()?;
            let tx = connection.transaction()?;
            tx.pragma_update(None, "application_id", application)?;
            tx.pragma_update(None, "user_version", version)?;
            assert_eq!(TaskStore::stored_version(&tx).is_ok(), accepted);
        }
        Ok(())
    }
}
