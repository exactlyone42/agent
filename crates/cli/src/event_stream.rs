use crate::{cli_error::CliError, fail, open_store};
#[cfg(test)]
mod poll_timing;
use agent_platform_agent::store::{StoreAccess, TaskStore};
use std::{
    ffi::OsString,
    io::{self, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

pub struct EventStream {
    store: TaskStore,
    task_id: String,
    after_seq: i64,
    follow: bool,
}

impl EventStream {
    pub fn from_args(args: &[OsString]) -> Result<Self, CliError> {
        let follow = args.last().is_some_and(|value| value == "--follow");
        let positional = &args[..args.len() - usize::from(follow)];
        if !(2..=3).contains(&positional.len()) {
            return Err(fail(
                "usage",
                "用法：agent-platform events <任務編號> [已讀序號] [--follow]",
            ));
        }
        let task_id = positional[1]
            .to_str()
            .ok_or_else(|| fail("invalid_argument", "任務編號格式錯誤"))?
            .to_owned();
        let after_seq = positional
            .get(2)
            .map(|value| {
                value
                    .to_str()
                    .and_then(|value| value.parse::<i64>().ok())
                    .filter(|value| *value >= 0)
                    .ok_or_else(|| fail("invalid_argument", "事件位置須為非負整數"))
            })
            .transpose()?
            .unwrap_or(0);
        let store = open_store(StoreAccess::Read)?;
        store.status(&task_id)?;
        Ok(Self {
            store,
            task_id,
            after_seq,
            follow,
        })
    }

    pub async fn run(mut self) -> Result<(), CliError> {
        let stopped = Arc::new(AtomicBool::new(false));
        if self.follow {
            let signal = stopped.clone();
            ctrlc::try_set_handler(move || signal.store(true, Ordering::SeqCst))
                .map_err(|_| fail("signal_handler_failed", "無法接收停止讀取要求"))?;
        }
        let mut output = io::stdout().lock();
        self.read_page(&mut output).await?;
        while self.follow && !stopped.load(Ordering::SeqCst) {
            self.read_page(&mut output).await?;
        }
        Ok(())
    }

    async fn read_page(&mut self, output: &mut impl Write) -> Result<(), CliError> {
        let events = self.store.events(&self.task_id, self.after_seq)?;
        let empty = events.is_empty();
        for event in events {
            serde_json::to_writer(&mut *output, &event)
                .map_err(|_| fail("output_failed", "無法輸出任務事件"))?;
            output
                .write_all(b"\n")
                .and_then(|()| output.flush())
                .map_err(|_| fail("output_failed", "無法輸出任務事件"))?;
            self.after_seq = event.seq;
        }
        if self.follow && empty {
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Ok(())
    }
}
