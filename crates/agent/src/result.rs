use crate::{input::open_result, store::ResultSnapshot, task::FILE_LIMIT};
use agent_harness::Message;
use serde::Serialize;
use std::{io::Read, path::PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum ResultError {
    #[error("缺少保存的任務規格，無法核對成果")]
    MissingSpec,
    #[error("成果路徑或檔案不符合要求")]
    InvalidArtifact,
    #[error("成果內容與保存紀錄不符，不能宣稱成功")]
    ContentMismatch,
    #[error("工具歷史不完整或互相矛盾，不能還原差異")]
    InvalidHistory,
}

#[derive(Serialize)]
pub struct TaskResult {
    pub schema_version: u32,
    pub task_id: String,
    pub recorded_state: String,
    pub input: PathBuf,
    pub output: PathBuf,
    pub before: Option<String>,
    pub after: String,
    pub expected_matches: Option<bool>,
    pub saved_output_matches: Option<bool>,
    pub verified: bool,
}

impl TaskResult {
    pub(crate) fn restore_file(
        snapshot: &ResultSnapshot,
    ) -> Result<Option<std::fs::File>, ResultError> {
        let spec = snapshot.spec.as_ref().ok_or(ResultError::MissingSpec)?;
        let Some(content) = Self::saved_content(snapshot, "write_output")? else {
            return match std::fs::symlink_metadata(&spec.task.output) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                _ => Err(ResultError::InvalidArtifact),
            };
        };
        if !spec.task.allow_write
            || spec
                .task
                .expected
                .as_ref()
                .is_some_and(|expected| expected != &content)
        {
            return Err(ResultError::ContentMismatch);
        }
        let mut file =
            open_result(&spec.task.output, FILE_LIMIT).map_err(|_| ResultError::InvalidArtifact)?;
        crate::verification::verify_file(&mut file, content.as_bytes())
            .map_err(|_| ResultError::ContentMismatch)?;
        Ok(Some(file))
    }

    pub fn inspect(snapshot: ResultSnapshot) -> Result<Self, ResultError> {
        let spec = snapshot.spec.as_ref().ok_or(ResultError::MissingSpec)?;
        let path = &spec.task.output;
        if !path.is_absolute()
            || std::fs::symlink_metadata(path)
                .map_err(|_| ResultError::InvalidArtifact)?
                .file_type()
                .is_symlink()
        {
            return Err(ResultError::InvalidArtifact);
        }
        let file = open_result(path, FILE_LIMIT).map_err(|_| ResultError::InvalidArtifact)?;
        let mut after = String::new();
        file.take(FILE_LIMIT + 1)
            .read_to_string(&mut after)
            .map_err(|_| ResultError::InvalidArtifact)?;
        if after.len() as u64 > FILE_LIMIT {
            return Err(ResultError::InvalidArtifact);
        }
        let expected_matches = spec
            .task
            .expected
            .as_ref()
            .map(|expected| *expected == after);
        let before = Self::saved_content(&snapshot, "read_input")?;
        let saved_output = Self::saved_content(&snapshot, "write_output")?;
        let saved_output_matches = saved_output.as_ref().map(|saved| saved == &after);
        if expected_matches == Some(false) || saved_output_matches == Some(false) {
            return Err(ResultError::ContentMismatch);
        }
        let settled = snapshot
            .session
            .as_ref()
            .is_some_and(|s| s.completed && !s.unresolved_effect);
        let verified = snapshot.record.recorded_state == "succeeded"
            && expected_matches == Some(true)
            && saved_output_matches == Some(true)
            && settled;
        Ok(Self {
            schema_version: 1,
            task_id: snapshot.record.task_id,
            recorded_state: snapshot.record.recorded_state,
            input: spec.task.input.clone(),
            output: path.clone(),
            before,
            after,
            expected_matches,
            saved_output_matches,
            verified,
        })
    }

    fn saved_content(snapshot: &ResultSnapshot, name: &str) -> Result<Option<String>, ResultError> {
        let Some(session) = &snapshot.session else {
            return Ok(None);
        };
        let values = snapshot
            .successful_calls
            .iter()
            .filter(|call| call.tool_name == name)
            .map(|receipt| Self::paired_content(&session.messages, receipt))
            .collect::<Result<Vec<_>, _>>()?;
        if values.windows(2).any(|pair| pair[0] != pair[1]) {
            return Err(ResultError::InvalidHistory);
        }
        Ok(values.into_iter().next())
    }

    fn paired_content(
        messages: &[Message],
        receipt: &crate::store::SavedToolCall,
    ) -> Result<String, ResultError> {
        let args: serde_json::Value =
            serde_json::from_str(&receipt.args_json).map_err(|_| ResultError::InvalidHistory)?;
        let calls: Vec<_> = messages
            .iter()
            .filter_map(|message| match message {
                Message::Assistant { calls, .. } => Some(calls),
                _ => None,
            })
            .flatten()
            .filter(|call| call.id == receipt.call_id)
            .collect();
        if calls.len() != 1
            || calls[0].name != receipt.tool_name
            || calls[0].args != args
            || calls[0].bad_args.is_some()
        {
            return Err(ResultError::InvalidHistory);
        }
        let results: Vec<_> = messages
            .iter()
            .filter_map(|message| match message {
                Message::Tool {
                    call_id,
                    content,
                    is_error: false,
                } if call_id == &receipt.call_id => Some(content),
                _ => None,
            })
            .collect();
        if results.len() != 1 {
            return Err(ResultError::InvalidHistory);
        }
        let value = if receipt.tool_name == "read_input" {
            results[0].clone()
        } else {
            args.get("content")
                .and_then(|value| value.as_str())
                .ok_or(ResultError::InvalidHistory)?
                .to_owned()
        };
        if value.len() as u64 > FILE_LIMIT {
            return Err(ResultError::InvalidHistory);
        }
        Ok(value)
    }
}
