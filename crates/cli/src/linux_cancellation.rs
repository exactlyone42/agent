use crate::cli_error::CliError;
use agent_platform_agent::{
    store::TaskStore, task_process::TaskProcess, task_session::TaskSession,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub struct LinuxCancellation {
    force_after: Instant,
    attempted: bool,
    process: Option<TaskProcess>,
}

impl LinuxCancellation {
    pub fn new() -> Self {
        Self {
            force_after: Instant::now() + Duration::from_secs(2),
            attempted: false,
            process: None,
        }
    }

    pub fn advance(
        &mut self,
        store: Arc<Mutex<TaskStore>>,
        task_id: &str,
        generation: i64,
    ) -> Result<(), CliError> {
        if let Some(process) = &self.process {
            TaskSession::settle_terminated(store, process)?;
            return Ok(());
        }
        if self.attempted || Instant::now() < self.force_after {
            return Ok(());
        }
        self.attempted = true;
        self.process = TaskSession::capture_process(store, task_id, generation)?;
        if let Some(process) = &self.process {
            process
                .terminate()
                .map_err(agent_platform_agent::store::StoreError::from)?;
        }
        Ok(())
    }
}
