use crate::{
    store::{APPROVAL_TTL_SECONDS, Claim, StoreError, TOOL_CALL_LIMIT, TaskStore},
    task::CancellationToken,
};
use rig_agent::agent::{
    AgentHook, DispatchAction, DispatchEvent, HookContext, OutcomeAction, OutcomeEvent,
};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

#[derive(Clone)]
pub struct ApprovalGate {
    store: Arc<Mutex<TaskStore>>,
    task_id: String,
    cancellation: CancellationToken,
    manual: bool,
    denied: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}

impl ApprovalGate {
    pub fn new(
        store: Arc<Mutex<TaskStore>>,
        task_id: String,
        cancellation: CancellationToken,
        manual: bool,
    ) -> Self {
        Self {
            store,
            task_id,
            cancellation,
            manual,
            denied: Arc::new(AtomicBool::new(false)),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn was_denied(&self) -> bool {
        self.denied.load(Ordering::SeqCst)
    }
}

impl AgentHook for ApprovalGate {
    async fn on_dispatch(&self, _: &HookContext, event: DispatchEvent<'_>) -> DispatchAction {
        let Some(name) = event.tool_name() else {
            return DispatchAction::proceed();
        };
        if self.cancellation.is_cancelled() {
            return DispatchAction::stop("任務已要求停止");
        }
        let (Some(call_id), Some(args)) = (event.call_id, event.tool_args()) else {
            return DispatchAction::stop("工具缺少識別或參數，不能執行");
        };
        let prepared = match self.store.lock() {
            Ok(mut store) => {
                if self.calls.fetch_add(1, Ordering::SeqCst) >= TOOL_CALL_LIMIT as usize {
                    return DispatchAction::stop("工具呼叫已達任務上限，不能繼續執行");
                }
                store.prepare_intent(
                    &self.task_id,
                    event.turn as i64,
                    &call_id.to_string(),
                    name,
                    args,
                )
            }
            Err(_) => return DispatchAction::stop("任務紀錄狀態異常，不能執行工具"),
        };
        let prepared = match prepared {
            Ok(prepared) => prepared,
            Err(StoreError::InvalidToolCall) => {
                let recorded = match self.store.lock() {
                    Ok(mut store) => store.record_rejection(&self.task_id, &event.id.to_string()),
                    Err(_) => return DispatchAction::stop("任務紀錄狀態異常，不能保存拒絕紀錄"),
                };
                if recorded.is_err() {
                    return DispatchAction::stop("無法保存工具拒絕紀錄，任務停止");
                }
                return DispatchAction::skip("工具參數或授權範圍不符合限制");
            }
            Err(_) => return DispatchAction::stop("無法保存工具意圖，未執行工具"),
        };
        let deadline =
            tokio::time::Instant::now() + Duration::from_secs(APPROVAL_TTL_SECONDS as u64);
        loop {
            if self.cancellation.is_cancelled() {
                return DispatchAction::stop("任務已要求停止");
            }
            if tokio::time::Instant::now() >= deadline {
                self.denied.store(true, Ordering::SeqCst);
                return DispatchAction::stop("等待批准逾時，未執行工具");
            }
            let claim = match self.store.lock() {
                Ok(mut store) => store.claim(&prepared),
                Err(_) => return DispatchAction::stop("任務紀錄狀態異常，不能執行工具"),
            };
            match claim {
                Ok(Claim::Granted) => return DispatchAction::proceed(),
                Ok(Claim::Denied) if self.manual => {
                    self.denied.store(true, Ordering::SeqCst);
                    return DispatchAction::stop("工具未獲批准或批准已失效");
                }
                Ok(Claim::Denied) => return DispatchAction::skip("使用者未授權此工具操作"),
                Err(StoreError::Database(rusqlite::Error::SqliteFailure(code, _)))
                    if code.code == rusqlite::ErrorCode::DatabaseBusy => {}
                Err(_) => return DispatchAction::stop("無法領用批准，未執行工具"),
                Ok(Claim::Pending) => {}
            }
            tokio::select! {
                _ = self.cancellation.cancelled() => return DispatchAction::stop("任務已要求停止"),
                _ = tokio::time::sleep(Duration::from_millis(200)) => {}
            }
        }
    }

    async fn on_outcome(&self, _: &HookContext, event: OutcomeEvent<'_>) -> OutcomeAction {
        let Some(result) = event.tool_result() else {
            return OutcomeAction::proceed();
        };
        if result.is_skipped() {
            return OutcomeAction::proceed();
        }
        let Some(call_id) = event.call_id else {
            return OutcomeAction::stop("工具結果缺少識別，不能確認成果");
        };
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            let saved = match self.store.lock() {
                Ok(mut store) => store.receipt(
                    &self.task_id,
                    event.turn as i64,
                    &call_id.to_string(),
                    result.is_success(),
                ),
                Err(_) => return OutcomeAction::stop("任務紀錄狀態異常，請核對已執行的動作"),
            };
            match saved {
                Ok(()) => return OutcomeAction::proceed(),
                Err(StoreError::Database(rusqlite::Error::SqliteFailure(code, _)))
                    if code.code == rusqlite::ErrorCode::DatabaseBusy
                        && tokio::time::Instant::now() < deadline => {}
                Err(_) => return OutcomeAction::stop("無法保存工具結果，請核對已執行的動作"),
            }
            tokio::select! {
                _ = self.cancellation.cancelled() => return OutcomeAction::stop("任務已要求停止；工具結果尚未保存"),
                _ = tokio::time::sleep(Duration::from_millis(200)) => {}
            }
        }
    }
}
