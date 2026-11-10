use super::{StoreError, TOOL_CALL_LIMIT, TaskRecord, TaskSpec, digest::digest};
use rusqlite::OptionalExtension;

pub const SPEC_LIMIT: usize = 131_072;
pub const SESSION_LIMIT: usize = 33_554_432;
const ARGS_LIMIT: usize = 6 * crate::task::FILE_LIMIT as usize + 1024;

pub struct SavedToolCall {
    pub call_id: String,
    pub tool_name: String,
    pub args_json: String,
}

impl SavedToolCall {
    fn validate_hash(&self, hash: &str) -> Result<(), StoreError> {
        if self.args_json.len() > ARGS_LIMIT || digest(&self.args_json) != hash {
            return Err(StoreError::InvalidToolCall);
        }
        Ok(())
    }
}

pub struct ResultSnapshot {
    pub record: TaskRecord,
    pub spec: Option<TaskSpec>,
    pub session: Option<agent_harness::Session>,
    pub successful_calls: Vec<SavedToolCall>,
}

impl super::TaskStore {
    pub fn result_snapshot(&self, task_id: &str) -> Result<ResultSnapshot, StoreError> {
        if uuid::Uuid::parse_str(task_id).is_err() {
            return Err(StoreError::InvalidSpec);
        }

        let tx = self.connection.unchecked_transaction()?;
        let record = self.status(task_id)?;

        if self.schema_version == 1 {
            return Ok(ResultSnapshot {
                record,
                spec: None,
                session: None,
                successful_calls: Vec::new(),
            });
        }

        let spec_row: Option<(String, String)> = tx
            .query_row(
                "SELECT substr(spec_json,1,?2), spec_hash FROM task_specs WHERE task_id = ?1",
                rusqlite::params![task_id, SPEC_LIMIT as i64 + 1],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;

        let spec = spec_row
            .map(|(json, hash)| Self::parse_result_spec(json, hash))
            .transpose()?;

        if self.schema_version < 4 {
            return Ok(ResultSnapshot {
                record,
                spec,
                session: None,
                successful_calls: Vec::new(),
            });
        }

        let session_raw: Option<(String, i64, i64)> = tx
            .query_row(
                "SELECT substr(session_json, 1, ?2), revision, generation FROM sessions WHERE task_id = ?1",
                rusqlite::params![task_id, SESSION_LIMIT as i64 + 1],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .optional()?;

        let session = session_raw
            .map(|(json, revision, generation)| {
                Self::parse_result_session(json, revision, generation, record.generation)
            })
            .transpose()?;

        let successful_calls = Self::result_calls(&tx, task_id, record.generation)?;
        Ok(ResultSnapshot {
            record,
            spec,
            session,
            successful_calls,
        })
    }

    fn result_calls(
        tx: &rusqlite::Transaction<'_>,
        task_id: &str,
        generation: i64,
    ) -> Result<Vec<SavedToolCall>, StoreError> {
        let mut query = tx.prepare(
            "SELECT call_id,tool_name,substr(args_json,1,?3),args_hash FROM tool_intents WHERE task_id=?1 AND generation<=?2 AND status='succeeded' ORDER BY generation,step_id,intent_id LIMIT ?4",
        )?;
        let rows = query.query_map(
            rusqlite::params![
                task_id,
                generation,
                ARGS_LIMIT as i64 + 1,
                TOOL_CALL_LIMIT + 1
            ],
            |row| {
                Ok((
                    SavedToolCall {
                        call_id: row.get(0)?,
                        tool_name: row.get(1)?,
                        args_json: row.get(2)?,
                    },
                    row.get::<_, String>(3)?,
                ))
            },
        )?;
        let mut calls = Vec::new();
        for row in rows {
            let (call, hash) = row?;
            call.validate_hash(&hash)?;
            calls.push(call);
        }
        if calls.len() as i64 > TOOL_CALL_LIMIT {
            return Err(StoreError::InvalidToolCall);
        }
        Ok(calls)
    }

    fn parse_result_spec(json: String, hash: String) -> Result<TaskSpec, StoreError> {
        if json.len() > SPEC_LIMIT || digest(&json) != hash {
            return Err(StoreError::InvalidSpec);
        }
        serde_json::from_str(&json).map_err(|_| StoreError::InvalidSpec)
    }

    fn parse_result_session(
        json: String,
        revision: i64,
        generation: i64,
        expected_generation: i64,
    ) -> Result<agent_harness::Session, StoreError> {
        if json.len() > SESSION_LIMIT {
            return Err(StoreError::InvalidSpec);
        }
        if generation != expected_generation {
            return Err(StoreError::Conflict);
        }
        let parsed: agent_harness::Session =
            serde_json::from_str(&json).map_err(|_| StoreError::InvalidSpec)?;
        if i64::try_from(parsed.revision).ok() != Some(revision) {
            return Err(StoreError::InvalidSpec);
        }
        Ok(parsed)
    }
}
