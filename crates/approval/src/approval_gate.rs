use crate::{ApprovalStore, Claim, Fault, Messages, Policy};
use agent_harness::{
    CancellationToken, Dispatch, DispatchAction, Hook, Outcome, OutcomeAction, ToolCall, ToolResult,
};
use std::{
    ops::ControlFlow,
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

    fn admit(&self) -> Option<DispatchAction> {
        if self.cancellation.is_cancelled() {
            return Some(self.stop(&self.messages.cancelled));
        }
        if self.calls.fetch_add(1, Ordering::SeqCst) >= self.policy.max_calls {
            return Some(self.stop(&self.messages.call_limit));
        }
        None
    }

    fn prepare(&self, event: &Dispatch<'_>) -> Result<S::Prepared, DispatchAction> {
        match self.store.prepare(event.turn, event.call) {
            Ok(prepared) => Ok(prepared),
            Err(Fault::Invalid) => Err(self.reject(event.call)),
            Err(_) => Err(self.stop(&self.messages.intent_failed)),
        }
    }

    fn reject(&self, call: &ToolCall) -> DispatchAction {
        match self.store.reject(call) {
            Ok(()) => DispatchAction::Skip(self.messages.invalid_call.clone()),
            Err(_) => self.stop(&self.messages.rejection_unrecorded),
        }
    }

    fn poll(&self, prepared: &S::Prepared, deadline: Instant) -> ControlFlow<DispatchAction> {
        let messages = &self.messages;
        if self.cancellation.is_cancelled() {
            return ControlFlow::Break(self.stop(&messages.cancelled));
        }
        if Instant::now() >= deadline {
            return ControlFlow::Break(self.deny_and_stop(&messages.approval_timeout));
        }
        match self.store.claim(prepared) {
            Ok(Claim::Granted) => ControlFlow::Break(DispatchAction::Proceed),
            Ok(Claim::Denied) if self.policy.manual => {
                ControlFlow::Break(self.deny_and_stop(&messages.approval_denied))
            }
            Ok(Claim::Denied) => {
                ControlFlow::Break(DispatchAction::Skip(messages.not_authorized.clone()))
            }
            Ok(Claim::Pending) | Err(Fault::Busy) => ControlFlow::Continue(()),
            Err(_) => ControlFlow::Break(self.stop(&messages.claim_failed)),
        }
    }

    fn save(&self, event: &Outcome<'_>, deadline: Instant) -> ControlFlow<OutcomeAction> {
        let saved = self.store.receipt(
            event.turn,
            event.call,
            event.result == ToolResult::Succeeded,
        );
        match saved {
            Ok(()) => ControlFlow::Break(OutcomeAction::Proceed),
            Err(Fault::Busy) if Instant::now() < deadline => self.keep_saving(),
            Err(_) => ControlFlow::Break(OutcomeAction::Stop(self.messages.receipt_failed.clone())),
        }
    }

    fn keep_saving(&self) -> ControlFlow<OutcomeAction> {
        if self.cancellation.is_cancelled() {
            return ControlFlow::Break(OutcomeAction::Stop(
                self.messages.receipt_cancelled.clone(),
            ));
        }
        ControlFlow::Continue(())
    }

    async fn pause(&self) {
        tokio::select! {
            _ = self.cancellation.cancelled() => {}
            _ = sleep(POLL) => {}
        }
    }
}

impl<S: ApprovalStore> Hook for ApprovalGate<S> {
    async fn on_dispatch(&self, event: Dispatch<'_>) -> DispatchAction {
        if let Some(action) = self.admit() {
            return action;
        }
        let prepared = match self.prepare(&event) {
            Ok(prepared) => prepared,
            Err(action) => return action,
        };
        let deadline = Instant::now() + self.policy.approval_wait;
        loop {
            match self.poll(&prepared, deadline) {
                ControlFlow::Break(action) => return action,
                ControlFlow::Continue(()) => self.pause().await,
            }
        }
    }

    async fn on_outcome(&self, event: Outcome<'_>) -> OutcomeAction {
        let deadline = Instant::now() + self.policy.receipt_wait;
        loop {
            match self.save(&event, deadline) {
                ControlFlow::Break(action) => return action,
                ControlFlow::Continue(()) => self.pause().await,
            }
        }
    }
}
