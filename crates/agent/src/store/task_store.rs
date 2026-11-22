use crate::prelude::*;
use crate::task::{ArtifactState, TaskError, TaskReport};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;

#[derive(Clone, Copy, PartialEq)]
pub enum StoreAccess {
    Read,
    Write,
    Create,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("could not save or read the task record")]
    Database(#[from] rusqlite::Error),
    #[error("the task database version or settings are incompatible")]
    Incompatible,
    #[error("the task was not found")]
    NotFound,
    #[error("the task record changed; a stale state cannot be saved or rerun")]
    Conflict,
    #[error("the event cursor must not be negative")]
    InvalidCursor,
    #[error("the task specification is outside the limits")]
    InvalidSpec,
    #[error("the tool name or arguments are outside the limits")]
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
    pub(crate) connection: Connection,
    pub(crate) schema_version: i64,
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
        if rusqlite::version_number() < MIN_SQLITE_VERSION {
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
        connection.busy_timeout(BUSY_TIMEOUT)?;
        let version: i64 =
            connection.pragma_query_value(None, PRAGMA_USER_VERSION, |r| r.get(0))?;
        let application: i64 =
            connection.pragma_query_value(None, PRAGMA_APPLICATION_ID, |r| r.get(0))?;
        if !matches!(
            (application, version),
            (UNSET_APPLICATION_ID, UNSET_USER_VERSION)
                | (APPLICATION_ID, FIRST_SCHEMA_VERSION..=SCHEMA_VERSION)
        ) {
            return Err(StoreError::Incompatible);
        }
        if version == UNSET_USER_VERSION && !create {
            return Err(StoreError::Incompatible);
        }
        if access == StoreAccess::Write && version != SCHEMA_VERSION {
            return Err(StoreError::Incompatible);
        }
        if create && version < SCHEMA_VERSION {
            Self::migrate(&mut connection)?;
        }
        let journal: String = if create {
            connection.pragma_update_and_check(
                None,
                PRAGMA_JOURNAL_MODE,
                JOURNAL_MODE_WAL,
                |r| r.get(0),
            )?
        } else {
            connection.pragma_query_value(None, PRAGMA_JOURNAL_MODE, |r| r.get(0))?
        };
        connection.pragma_update(None, PRAGMA_SYNCHRONOUS, SYNCHRONOUS_MODE_FULL)?;
        connection.pragma_update(None, PRAGMA_FOREIGN_KEYS, true)?;
        connection.pragma_update(None, PRAGMA_TRUSTED_SCHEMA, false)?;
        let synchronous: i64 =
            connection.pragma_query_value(None, PRAGMA_SYNCHRONOUS, |r| r.get(0))?;
        let foreign_keys: i64 =
            connection.pragma_query_value(None, PRAGMA_FOREIGN_KEYS, |r| r.get(0))?;
        if (create && journal != JOURNAL_MODE_WAL)
            || synchronous != SYNCHRONOUS_FULL
            || foreign_keys != FOREIGN_KEYS_ON
        {
            return Err(StoreError::Incompatible);
        }
        let schema_version =
            connection.pragma_query_value(None, PRAGMA_USER_VERSION, |r| r.get(0))?;
        Ok(Self {
            connection,
            schema_version,
        })
    }

    fn migrate(connection: &mut Connection) -> Result<(), StoreError> {
        connection.pragma_update(None, PRAGMA_FOREIGN_KEYS, false)?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut version = Self::stored_version(&transaction)?;
        if version == FIRST_SCHEMA_VERSION {
            transaction.execute_batch(include_str!("../../../../sql/store_v2.sql"))?;
            version = SECOND_SCHEMA_VERSION;
        }
        if version == SECOND_SCHEMA_VERSION {
            transaction.execute_batch(include_str!("../../../../sql/store_v3.sql"))?;
        }
        transaction.pragma_update(None, PRAGMA_USER_VERSION, SCHEMA_VERSION)?;
        transaction.commit()?;
        Ok(())
    }

    fn stored_version(transaction: &Transaction<'_>) -> Result<i64, StoreError> {
        let version: i64 =
            transaction.pragma_query_value(None, PRAGMA_USER_VERSION, |r| r.get(0))?;
        let application: i64 =
            transaction.pragma_query_value(None, PRAGMA_APPLICATION_ID, |r| r.get(0))?;
        if (application, version) == (UNSET_APPLICATION_ID, UNSET_USER_VERSION) {
            return Self::install_first_version(transaction);
        }
        if application != APPLICATION_ID
            || !(FIRST_SCHEMA_VERSION..=SCHEMA_VERSION).contains(&version)
        {
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
        transaction.execute_batch(include_str!("../../../../sql/store_v1.sql"))?;
        transaction.pragma_update(None, PRAGMA_APPLICATION_ID, APPLICATION_ID)?;
        Ok(FIRST_SCHEMA_VERSION)
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
        if spec_json
            .as_ref()
            .is_some_and(|value| value.len() > MAX_SPEC_BYTES)
        {
            return Err(StoreError::InvalidSpec);
        }
        let task_id = Uuid::new_v4().to_string();
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let (generation, seq) = transaction.query_row(
            "INSERT INTO tasks (task_id, recorded_state, last_seq) VALUES (?1, ?2, ?3) RETURNING generation, last_seq",
            params![task_id, TaskState::Running, FIRST_SEQ],
            |r| Ok((r.get::<_, i64>("generation")?, r.get::<_, i64>("last_seq")?)),
        )?;
        transaction.execute(
            "INSERT INTO events (event_id, task_id, seq, event_type) VALUES (?1, ?2, ?3, ?4)",
            params![Uuid::new_v4().to_string(), task_id, seq, EventKind::Started],
        )?;
        if let Some(spec_json) = spec_json {
            transaction.execute(
                "INSERT INTO task_specs(task_id,spec_json,spec_hash) VALUES (?1,?2,?3)",
                params![task_id, spec_json, digest(&spec_json)],
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
                ArtifactState::Verified => TaskState::Succeeded,
                ArtifactState::AwaitingReview => TaskState::NeedsReview,
            },
            Err(TaskError::Cancelled { .. }) => TaskState::Cancelled,
            Err(TaskError::Timeout | TaskError::Harness(_) | TaskError::State) => {
                TaskState::OutcomeUnknown
            }
            Err(_) => TaskState::Failed,
        };
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let unresolved: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM tool_intents WHERE task_id=?1 AND status=?2)",
            params![task_id, IntentStatus::Dispatched],
            |r| r.get(0),
        )?;
        let state = if unresolved {
            TaskState::OutcomeUnknown
        } else {
            state
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
            &format!("UPDATE tasks SET recorded_state = ?1, last_seq = last_seq + 1, artifact_created = ?4,
             updated_at = {TIMESTAMP_NOW}
             WHERE task_id = ?2 AND recorded_state IN (?5,?6) AND generation = ?3 RETURNING last_seq"),
            params![state, task_id, expected_generation, artifact_created, TaskState::Running, TaskState::WaitingApproval], |r| r.get(0),
        ).optional()?;
        let Some(seq) = seq else {
            return Err(Self::missing_cause(&transaction, task_id)?);
        };
        transaction.execute(
            "INSERT INTO events (event_id, task_id, seq, event_type) VALUES (?1, ?2, ?3, ?4)",
            params![Uuid::new_v4().to_string(), task_id, seq, state],
        )?;
        close_intents(&transaction, task_id)?;
        transaction.execute(
            "UPDATE tool_intents SET status=?2 WHERE task_id=?1 AND status=?3",
            params![task_id, IntentStatus::Cancelled, IntentStatus::Prepared],
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
                        task_id: r.get("task_id")?,
                        schema_version: r.get("schema_version")?,
                        generation: r.get("generation")?,
                        recorded_state: r.get("recorded_state")?,
                        last_seq: r.get("last_seq")?,
                        updated_at: r.get("updated_at")?,
                        artifact_created: r.get("artifact_created")?,
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
        let related = if self.schema_version == FIRST_SCHEMA_VERSION {
            "NULL"
        } else {
            "related_id"
        };
        let mut query = self.connection.prepare(&format!(
            "SELECT event_id, task_id, schema_version, generation, seq, event_type, created_at, {related} AS related_id
             FROM events WHERE task_id = ?1 AND seq > ?2 ORDER BY seq LIMIT ?3"
        ))?;
        let rows = query.query_map(params![task_id, after_seq, EVENT_PAGE_SIZE], |r| {
            Ok(TaskEvent {
                event_id: r.get("event_id")?,
                task_id: r.get("task_id")?,
                schema_version: r.get("schema_version")?,
                generation: r.get("generation")?,
                seq: r.get("seq")?,
                event_type: r.get("event_type")?,
                created_at: r.get("created_at")?,
                related_id: r.get("related_id")?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
