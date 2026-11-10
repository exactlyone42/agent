use crate::instruction_text::instruction_text;
use crate::store::{StoreError, TaskRecord, TaskSpec, TaskStore};
use agent_harness::{Message, Session, SessionError, SessionStore};
use rusqlite::{OptionalExtension, TransactionBehavior, params};
use std::fs::{File, OpenOptions};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct TaskSession {
    store: Arc<Mutex<TaskStore>>,
    task_id: String,
    generation: i64,
    _ownership: File,
}

impl TaskSession {
    pub fn stopped(
        store: Arc<Mutex<TaskStore>>,
        task_id: String,
        generation: i64,
    ) -> Result<Option<TaskRecord>, StoreError> {
        let owner = match Self::new(store, task_id.clone(), generation) {
            Ok(owner) => owner,
            Err(SessionError::Conflict) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let record = owner
            .store
            .lock()
            .map_err(|_| SessionError::Storage)?
            .status(&task_id)?;
        if record.generation != generation {
            return Err(StoreError::Conflict);
        }
        if matches!(
            record.recorded_state.as_str(),
            "cancelled" | "outcome_unknown"
        ) {
            return Ok(Some(record));
        }
        Ok(None)
    }

    pub fn resume(
        store: Arc<Mutex<TaskStore>>,
        task_id: String,
        expected_spec: &TaskSpec,
    ) -> Result<(Self, Option<File>), StoreError> {
        let generation = store
            .lock()
            .map_err(|_| SessionError::Storage)?
            .status(&task_id)?
            .generation;
        let mut session = Self::new(store.clone(), task_id.clone(), generation)?;
        let (generation, artifact) = store
            .lock()
            .map_err(|_| SessionError::Storage)?
            .resume_task(&task_id, generation, expected_spec)?;
        session.generation = generation;
        Ok((session, artifact))
    }

    pub fn generation(&self) -> i64 {
        self.generation
    }

    pub fn new(
        store: Arc<Mutex<TaskStore>>,
        task_id: String,
        generation: i64,
    ) -> Result<Self, SessionError> {
        let ownership = Self::ownership_file(&store, &task_id)?;
        ownership.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => SessionError::Conflict,
            std::fs::TryLockError::Error(_) => SessionError::Storage,
        })?;
        Ok(Self {
            store,
            task_id,
            generation,
            _ownership: ownership,
        })
    }

    fn ownership_file(store: &Arc<Mutex<TaskStore>>, task_id: &str) -> Result<File, SessionError> {
        uuid::Uuid::parse_str(task_id).map_err(|_| SessionError::Conflict)?;
        let path = {
            let locked = store.lock().map_err(|_| SessionError::Storage)?;
            let path = locked.connection.path().ok_or(SessionError::Storage)?;
            Path::new(path).with_extension(format!("{task_id}.lock"))
        };
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let ownership = options.open(path).map_err(|_| SessionError::Storage)?;
        if !ownership
            .metadata()
            .map_err(|_| SessionError::Storage)?
            .is_file()
        {
            return Err(SessionError::Storage);
        }
        Ok(ownership)
    }
}

#[cfg(target_os = "linux")]
impl TaskSession {
    pub fn register_process(&self) -> Result<(), StoreError> {
        crate::task_process::TaskProcess::register(
            &self._ownership,
            &self.task_id,
            self.generation,
        )?;
        Ok(())
    }

    pub fn capture_process(
        store: Arc<Mutex<TaskStore>>,
        task_id: &str,
        generation: i64,
    ) -> Result<Option<crate::task_process::TaskProcess>, StoreError> {
        let record = store
            .lock()
            .map_err(|_| SessionError::Storage)?
            .status(task_id)?;
        if record.generation != generation || record.recorded_state != "cancel_requested" {
            return Err(StoreError::Conflict);
        }
        let file = Self::ownership_file(&store, task_id)?;
        Ok(crate::task_process::TaskProcess::capture(
            file, task_id, generation,
        )?)
    }

    pub fn settle_terminated(
        store: Arc<Mutex<TaskStore>>,
        process: &crate::task_process::TaskProcess,
    ) -> Result<bool, StoreError> {
        if !process.exited()? {
            return Ok(false);
        }
        let owner = match Self::new(store, process.task_id.clone(), process.generation) {
            Ok(owner) => owner,
            Err(SessionError::Conflict) => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        let mut store = owner.store.lock().map_err(|_| SessionError::Storage)?;
        let record = store.status(&process.task_id)?;
        if record.generation != process.generation {
            return Err(StoreError::Conflict);
        }
        if matches!(
            record.recorded_state.as_str(),
            "cancelled" | "outcome_unknown"
        ) {
            return Ok(true);
        }
        if record.recorded_state != "cancel_requested" {
            return Err(StoreError::Conflict);
        }
        let (unclear, created): (bool, bool) = store.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM tool_intents WHERE task_id=?1 AND (status IN ('dispatched','outcome_unknown') OR (tool_name='write_output' AND status='failed'))),
             EXISTS(SELECT 1 FROM tool_intents WHERE task_id=?1 AND tool_name='write_output' AND status='succeeded')",
            [&process.task_id], |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        let result = if unclear {
            Err(crate::task::TaskError::State)
        } else {
            Err(crate::task::TaskError::Cancelled {
                artifact_created: created,
            })
        };
        store.finish(&process.task_id, process.generation, &result)?;
        Ok(true)
    }
}

impl SessionStore for TaskSession {
    fn load(&self) -> Result<Option<Session>, SessionError> {
        let store = self.store.lock().map_err(|_| SessionError::Storage)?;
        let json: Option<String> = store
            .connection
            .query_row(
                "SELECT session_json FROM sessions WHERE task_id=?1 AND generation=?2",
                params![self.task_id, self.generation],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| SessionError::Storage)?;
        json.map(|json| serde_json::from_str(&json).map_err(|_| SessionError::Storage))
            .transpose()
    }

    fn save(&self, session: &Session) -> Result<u64, SessionError> {
        let revision = session
            .revision
            .checked_add(1)
            .ok_or(SessionError::Storage)?;
        let saved = Session {
            revision,
            ..session.clone()
        };
        let json = serde_json::to_string(&saved).map_err(|_| SessionError::Storage)?;
        if json.len() > 32 * 1024 * 1024 {
            return Err(SessionError::Storage);
        }
        let mut store = self.store.lock().map_err(|_| SessionError::Storage)?;
        store
            .connection
            .busy_timeout(Duration::from_secs(2))
            .map_err(|_| SessionError::Storage)?;
        let result = self.persist(&mut store, &saved, &json);
        store
            .connection
            .busy_timeout(Duration::from_millis(500))
            .map_err(|_| SessionError::Storage)?;
        result
    }

    fn inputs(&self, messages: &[Message]) -> Result<Vec<String>, SessionError> {
        let store = self.store.lock().map_err(|_| SessionError::Storage)?;
        let received = Self::received(&store.connection, &self.task_id)?;
        Ok(received
            .into_iter()
            .map(|(_, text)| text)
            .filter(|text| !messages.contains(&Message::User(text.clone())))
            .collect())
    }
}

impl TaskSession {
    fn received(
        connection: &rusqlite::Connection,
        task_id: &str,
    ) -> Result<Vec<(i64, String)>, SessionError> {
        let mut query = connection
            .prepare("SELECT seq,content FROM instructions WHERE task_id=?1 AND status='received' ORDER BY seq")
            .map_err(|_| SessionError::Storage)?;
        query
            .query_map([task_id], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    instruction_text(row.get(0)?, &row.get::<_, String>(1)?),
                ))
            })
            .and_then(|rows| rows.collect())
            .map_err(|_| SessionError::Storage)
    }

    fn mark_applied(
        &self,
        tx: &rusqlite::Transaction<'_>,
        saved: &Session,
    ) -> Result<(), SessionError> {
        let applied = Self::received(tx, &self.task_id)?
            .into_iter()
            .filter(|(_, text)| saved.messages.contains(&Message::User(text.clone())));
        for (seq, _) in applied {
            tx.execute(
                "UPDATE instructions SET status='applied' WHERE task_id=?1 AND seq=?2",
                params![self.task_id, seq],
            )
            .map_err(|_| SessionError::Storage)?;
        }
        Ok(())
    }

    fn persist(
        &self,
        store: &mut TaskStore,
        saved: &Session,
        json: &str,
    ) -> Result<u64, SessionError> {
        let tx = store
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| SessionError::Storage)?;
        let active: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM tasks WHERE task_id=?1 AND generation=?2 AND recorded_state IN ('running','waiting_approval','cancel_requested'))",
            params![self.task_id, self.generation], |row| row.get(0),
        ).map_err(|_| SessionError::Storage)?;
        if !active {
            return Err(SessionError::Conflict);
        }
        let current: Option<(i64, i64)> = tx
            .query_row(
                "SELECT generation,revision FROM sessions WHERE task_id=?1",
                [&self.task_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| SessionError::Storage)?;
        if current.is_some_and(|(generation, revision)| {
            generation != self.generation
                || u64::try_from(revision).ok() != Some(saved.revision - 1)
        }) || (current.is_none() && saved.revision != 1)
        {
            return Err(SessionError::Conflict);
        }
        tx.execute(
            "INSERT INTO sessions(task_id,generation,revision,session_json) VALUES (?1,?2,?3,?4)
             ON CONFLICT(task_id) DO UPDATE SET revision=excluded.revision,session_json=excluded.session_json",
            params![self.task_id,self.generation,i64::try_from(saved.revision).map_err(|_| SessionError::Storage)?,json],
        ).map_err(|_| SessionError::Storage)?;
        self.mark_applied(&tx, saved)?;
        tx.commit().map_err(|_| SessionError::Storage)?;
        Ok(saved.revision)
    }
}
