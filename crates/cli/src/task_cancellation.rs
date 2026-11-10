use crate::{cli_error::CliError, fail, open_store};
use agent_platform_agent::{
    store::{StoreAccess, TaskStore},
    task::CancellationToken,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{JoinHandle, park_timeout},
    time::Duration,
};

pub struct TaskCancellation {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<Result<(), CliError>>>,
}

impl TaskCancellation {
    pub fn start(
        task_id: String,
        generation: i64,
        cancellation: CancellationToken,
    ) -> Result<Self, CliError> {
        let reader = open_store(StoreAccess::Read)?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let worker = std::thread::Builder::new()
            .name("task-cancellation".into())
            .spawn(move || Self::watch(reader, task_id, generation, cancellation, worker_stop))
            .map_err(|_| {
                fail(
                    "cancellation_watch_failed",
                    "無法接收跨程序停止要求，任務未派工",
                )
            })?;
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }

    fn watch(
        reader: TaskStore,
        task_id: String,
        generation: i64,
        cancellation: CancellationToken,
        stop: Arc<AtomicBool>,
    ) -> Result<(), CliError> {
        while !stop.load(Ordering::SeqCst)
            && Self::observe(&reader, &task_id, generation, &cancellation)?
        {
            park_timeout(Duration::from_millis(100));
        }
        Ok(())
    }

    fn observe(
        reader: &TaskStore,
        task_id: &str,
        generation: i64,
        cancellation: &CancellationToken,
    ) -> Result<bool, CliError> {
        let status = reader
            .status(task_id)
            .inspect_err(|_| cancellation.cancel())?;
        let active = status.generation == generation
            && matches!(
                status.recorded_state.as_str(),
                "running" | "waiting_approval"
            );
        if !active {
            cancellation.cancel();
        }
        Ok(active)
    }

    pub fn stop(&mut self) -> Result<(), CliError> {
        self.stop.store(true, Ordering::SeqCst);
        let Some(worker) = self.worker.take() else {
            return Ok(());
        };
        worker.thread().unpark();
        worker.join().map_err(|_| {
            fail(
                "cancellation_watch_failed",
                "跨程序停止接收器已中斷，請核對任務結果",
            )
        })?
    }
}

impl Drop for TaskCancellation {
    fn drop(&mut self) {
        if let Err(error) = self.stop() {
            eprintln!("{}", error.line());
        }
    }
}
