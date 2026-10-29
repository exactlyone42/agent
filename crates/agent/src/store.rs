use crate::task::{ArtifactState, TaskError, TaskReport};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior, params,
};
use serde::Serialize;
use std::{path::Path, time::Duration};
use uuid::Uuid;
mod append;
mod approvals;
mod close_intents;
mod digest;
pub use approvals::{
    APPROVAL_TTL_SECONDS, ApprovalMode, ApprovalRecord, Claim, PreparedIntent, TOOL_CALL_LIMIT,
    TaskSpec,
};

const APPLICATION_ID: i64 = 0x41504754;
const SCHEMA_VERSION: i64 = 3;
pub const EVENT_PAGE_SIZE: i64 = 100;

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
    #[error("事件位置不可小於零")]
    InvalidCursor,
    #[error("任務規格不符合限制，不能繼續執行")]
    InvalidSpec,
    #[error("工具名稱或參數不符合限制")]
    InvalidToolCall,
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

pub struct TaskStore {
    connection: Connection,
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
        let state = if unresolved { "outcome_unknown" } else { state };
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
             WHERE task_id = ?2 AND recorded_state IN ('running','waiting_approval') AND generation = ?3 RETURNING last_seq",
            params![state, task_id, expected_generation, artifact_created], |r| r.get(0),
        ).optional()?;
        let Some(seq) = seq else {
            return Err(Self::missing_cause(&transaction, task_id)?);
        };
        transaction.execute(
            "INSERT INTO events (event_id, task_id, seq, event_type) VALUES (?1, ?2, ?3, ?4)",
            params![Uuid::new_v4().to_string(), task_id, seq, state],
        )?;
        close_intents::close_intents(&transaction, task_id)?;
        transaction.execute(
            "UPDATE tool_intents SET status='cancelled' WHERE task_id=?1 AND status='prepared'",
            [task_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn status(&self, task_id: &str) -> Result<TaskRecord, StoreError> {
        self.connection
            .query_row(
                "SELECT task_id, schema_version, generation, recorded_state, last_seq, updated_at, artifact_created
             FROM tasks WHERE task_id = ?1",
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
