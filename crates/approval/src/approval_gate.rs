use crate::{ApprovalStore, Claim, Fault, Messages, Policy};
use agent_harness::{
    CancellationToken, Dispatch, DispatchAction, Hook, Outcome, OutcomeAction, ToolResult,
};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::time::{Instant, sleep};

const POLL: Duration = Duration::from_millis(200);

pub struct ApprovalGate<S> {
    store: Arc<S>,
    cancellation: CancellationToken,
    policy: Policy,
    messages: Arc<Messages>,
    denied: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}

impl<S> Clone for ApprovalGate<S> {
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            cancellation: self.cancellation.clone(),
            policy: self.policy.clone(),
            messages: self.messages.clone(),
            denied: self.denied.clone(),
            calls: self.calls.clone(),
        }
    }
}

impl<S: ApprovalStore> ApprovalGate<S> {
    pub fn new(
        store: Arc<S>,
        cancellation: CancellationToken,
        policy: Policy,
        messages: Messages,
    ) -> Self {
        Self {
            store,
            cancellation,
            policy,
            messages: Arc::new(messages),
            denied: Arc::new(AtomicBool::new(false)),
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn was_denied(&self) -> bool {
        self.denied.load(Ordering::SeqCst)
    }

    fn stop(&self, why: &str) -> DispatchAction {
        DispatchAction::Stop(why.to_owned())
    }

    fn deny_and_stop(&self, why: &str) -> DispatchAction {
        self.denied.store(true, Ordering::SeqCst);
        self.stop(why)
    }
}

impl<S: ApprovalStore> Hook for ApprovalGate<S> {
    async fn on_dispatch(&self, event: Dispatch<'_>) -> DispatchAction {
        let call = event.call;
        let messages = &self.messages;
        if self.cancellation.is_cancelled() {
            return self.stop(&messages.cancelled);
        }
        if self.calls.fetch_add(1, Ordering::SeqCst) >= self.policy.max_calls {
            return self.stop(&messages.call_limit);
        }
        let prepared = match self.store.prepare(event.turn, call) {
            Ok(prepared) => prepared,
            Err(Fault::Invalid) => {
                return match self.store.reject(call) {
                    Ok(()) => DispatchAction::Skip(messages.invalid_call.clone()),
                    Err(_) => self.stop(&messages.rejection_unrecorded),
                };
            }
            Err(_) => return self.stop(&messages.intent_failed),
        };
        let deadline = Instant::now() + self.policy.approval_wait;
        loop {
            if self.cancellation.is_cancelled() {
                return self.stop(&messages.cancelled);
            }
            if Instant::now() >= deadline {
                return self.deny_and_stop(&messages.approval_timeout);
            }
            match self.store.claim(&prepared) {
                Ok(Claim::Granted) => return DispatchAction::Proceed,
                Ok(Claim::Denied) if self.policy.manual => {
                    return self.deny_and_stop(&messages.approval_denied);
                }
                Ok(Claim::Denied) => {
                    return DispatchAction::Skip(messages.not_authorized.clone());
                }
                Ok(Claim::Pending) | Err(Fault::Busy) => {}
                Err(_) => return self.stop(&messages.claim_failed),
            }
            tokio::select! {
                _ = self.cancellation.cancelled() => return self.stop(&messages.cancelled),
                _ = sleep(POLL) => {}
            }
        }
    }

    async fn on_outcome(&self, event: Outcome<'_>) -> OutcomeAction {
        let messages = &self.messages;
        let deadline = Instant::now() + self.policy.receipt_wait;
        loop {
            let saved = self.store.receipt(
                event.turn,
                event.call,
                event.result == ToolResult::Succeeded,
            );
            match saved {
                Ok(()) => return OutcomeAction::Proceed,
                Err(Fault::Busy) if Instant::now() < deadline => {}
                Err(_) => return OutcomeAction::Stop(messages.receipt_failed.clone()),
            }
            tokio::select! {
                _ = self.cancellation.cancelled() => {
                    return OutcomeAction::Stop(messages.receipt_cancelled.clone());
                }
                _ = sleep(POLL) => {}
            }
        }
    }
}
