use agent_harness::{
    CancellationToken, Dispatch, DispatchAction, Hook, Outcome, OutcomeAction, ToolCall, ToolResult,
};
use agent_harness_approval::{ApprovalGate, ApprovalStore, Claim, Fault, Messages, Policy};
use serde_json::json;
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

struct Fake {
    prepare: Result<(), Fault>,
    reject: Result<(), Fault>,
    claims: Mutex<VecDeque<Result<Claim, Fault>>>,
    receipts: Mutex<VecDeque<Result<(), Fault>>>,
    rejected: AtomicUsize,
    receipted: AtomicUsize,
}

impl Fake {
    fn new(claims: Vec<Result<Claim, Fault>>) -> Self {
        Self {
            prepare: Ok(()),
            reject: Ok(()),
            claims: Mutex::new(claims.into()),
            receipts: Mutex::new(VecDeque::new()),
            rejected: AtomicUsize::new(0),
            receipted: AtomicUsize::new(0),
        }
    }
}

impl ApprovalStore for Fake {
    type Prepared = ();

    fn prepare(&self, _: usize, _: &ToolCall) -> Result<(), Fault> {
        self.prepare
    }

    fn claim(&self, _: &()) -> Result<Claim, Fault> {
        self.claims
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(Claim::Pending))
    }

    fn receipt(&self, _: usize, _: &ToolCall, _: bool) -> Result<(), Fault> {
        self.receipted.fetch_add(1, Ordering::SeqCst);
        self.receipts.lock().unwrap().pop_front().unwrap_or(Ok(()))
    }

    fn reject(&self, _: &ToolCall) -> Result<(), Fault> {
        self.rejected.fetch_add(1, Ordering::SeqCst);
        self.reject
    }
}

fn call() -> ToolCall {
    ToolCall {
        id: "c1".into(),
        name: "read".into(),
        args: json!({}),
    }
}

fn gate(store: Fake, policy: Policy) -> (ApprovalGate<Fake>, Arc<Fake>, CancellationToken) {
    let store = Arc::new(store);
    let token = CancellationToken::new();
    let gate = ApprovalGate::new(store.clone(), token.clone(), policy, Messages::default());
    (gate, store, token)
}

async fn dispatch(gate: &ApprovalGate<Fake>) -> DispatchAction {
    gate.on_dispatch(Dispatch {
        turn: 0,
        call: &call(),
    })
    .await
}

async fn outcome(gate: &ApprovalGate<Fake>) -> OutcomeAction {
    gate.on_outcome(Outcome {
        turn: 0,
        call: &call(),
        result: ToolResult::Succeeded,
    })
    .await
}

fn is_stop(action: &DispatchAction) -> bool {
    matches!(action, DispatchAction::Stop(_))
}

fn is_skip(action: &DispatchAction) -> bool {
    matches!(action, DispatchAction::Skip(_))
}

#[tokio::test]
async fn granted_call_proceeds() {
    let (gate, _, _) = gate(Fake::new(vec![Ok(Claim::Granted)]), Policy::default());
    assert_eq!(dispatch(&gate).await, DispatchAction::Proceed);
    assert!(!gate.was_denied());
}

#[tokio::test]
async fn denied_call_is_skipped_when_preapproved() {
    let (gate, _, _) = gate(Fake::new(vec![Ok(Claim::Denied)]), Policy::default());
    assert!(is_skip(&dispatch(&gate).await));
    assert!(!gate.was_denied());
}

#[tokio::test]
async fn denied_call_stops_the_run_when_manual() {
    let policy = Policy {
        manual: true,
        ..Policy::default()
    };
    let (gate, _, _) = gate(Fake::new(vec![Ok(Claim::Denied)]), policy);
    assert!(is_stop(&dispatch(&gate).await));
    assert!(gate.was_denied());
}

#[tokio::test]
async fn invalid_call_is_skipped_and_recorded() {
    let mut fake = Fake::new(vec![]);
    fake.prepare = Err(Fault::Invalid);
    let (gate, store, _) = gate(fake, Policy::default());
    assert!(is_skip(&dispatch(&gate).await));
    assert_eq!(store.rejected.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn unrecorded_rejection_stops_the_run() {
    let mut fake = Fake::new(vec![]);
    fake.prepare = Err(Fault::Invalid);
    fake.reject = Err(Fault::Failed);
    let (gate, _, _) = gate(fake, Policy::default());
    assert!(is_stop(&dispatch(&gate).await));
}

#[tokio::test]
async fn failed_prepare_stops_without_claiming() {
    let mut fake = Fake::new(vec![Ok(Claim::Granted)]);
    fake.prepare = Err(Fault::Failed);
    let (gate, store, _) = gate(fake, Policy::default());
    assert!(is_stop(&dispatch(&gate).await));
    assert_eq!(store.claims.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn busy_claim_is_retried() {
    let (gate, _, _) = gate(
        Fake::new(vec![Err(Fault::Busy), Ok(Claim::Granted)]),
        Policy::default(),
    );
    let started = Instant::now();
    assert_eq!(dispatch(&gate).await, DispatchAction::Proceed);
    assert!(started.elapsed() >= Duration::from_millis(150));
}

#[tokio::test]
async fn failed_claim_stops_the_run() {
    let (gate, _, _) = gate(Fake::new(vec![Err(Fault::Failed)]), Policy::default());
    assert!(is_stop(&dispatch(&gate).await));
}

#[tokio::test]
async fn pending_approval_times_out_and_stops() {
    let policy = Policy {
        approval_wait: Duration::from_millis(300),
        ..Policy::default()
    };
    let (gate, _, _) = gate(Fake::new(vec![]), policy);
    assert!(is_stop(&dispatch(&gate).await));
    assert!(gate.was_denied());
}

#[tokio::test]
async fn cancelled_run_stops_before_preparing() {
    let (gate, _, token) = gate(Fake::new(vec![Ok(Claim::Granted)]), Policy::default());
    token.cancel();
    assert!(is_stop(&dispatch(&gate).await));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_while_waiting_stops() {
    let (gate, _, token) = gate(Fake::new(vec![]), Policy::default());
    let waiting = {
        let gate = gate.clone();
        tokio::spawn(async move { dispatch(&gate).await })
    };
    tokio::time::sleep(Duration::from_millis(100)).await;
    token.cancel();
    assert!(is_stop(&waiting.await.unwrap()));
}

#[tokio::test]
async fn call_limit_stops_the_extra_call() {
    let policy = Policy {
        max_calls: 1,
        ..Policy::default()
    };
    let (gate, _, _) = gate(
        Fake::new(vec![Ok(Claim::Granted), Ok(Claim::Granted)]),
        policy,
    );
    assert_eq!(dispatch(&gate).await, DispatchAction::Proceed);
    assert!(is_stop(&dispatch(&gate).await));
}

#[tokio::test]
async fn saved_receipt_proceeds() {
    let (gate, store, _) = gate(Fake::new(vec![]), Policy::default());
    assert_eq!(outcome(&gate).await, OutcomeAction::Proceed);
    assert_eq!(store.receipted.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn busy_receipt_is_retried() {
    let fake = Fake::new(vec![]);
    fake.receipts.lock().unwrap().push_back(Err(Fault::Busy));
    let (gate, store, _) = gate(fake, Policy::default());
    assert_eq!(outcome(&gate).await, OutcomeAction::Proceed);
    assert_eq!(store.receipted.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn failed_receipt_stops_the_run() {
    let fake = Fake::new(vec![]);
    fake.receipts.lock().unwrap().push_back(Err(Fault::Failed));
    let (gate, _, _) = gate(fake, Policy::default());
    assert!(matches!(outcome(&gate).await, OutcomeAction::Stop(_)));
}
