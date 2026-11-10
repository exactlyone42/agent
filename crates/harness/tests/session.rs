use agent_harness::{
    Agent, CancellationToken, MemorySessionStore, Message, Model, ModelError, ModelReply, Session,
    SessionError, SessionStore, ToolSpec, TurnStatus,
};

struct Done;

struct Continue;

impl Model for Continue {
    async fn complete(&self, _: &[Message], _: &[ToolSpec]) -> Result<ModelReply, ModelError> {
        Ok(ModelReply {
            message: Message::text("continue"),
            status: TurnStatus::Continue,
        })
    }
}

#[tokio::test]
async fn persisted_turn_budget_prevents_extra_turns_after_reopen() {
    let store = MemorySessionStore::default();
    let result = Agent::new(Continue)
        .session(&store)
        .max_turns(3)
        .run("work", &CancellationToken::new())
        .await;
    assert!(matches!(result, Err(agent_harness::RunError::TurnLimit)));
    let session = store.load().unwrap().unwrap();
    assert_eq!(session.next_turn, 3);
    assert_eq!(session.messages.len(), 4);
    let result = Agent::new(Done)
        .session(&store)
        .max_turns(3)
        .run("work", &CancellationToken::new())
        .await;
    assert!(matches!(result, Err(agent_harness::RunError::TurnLimit)));
    assert_eq!(store.load().unwrap().unwrap().messages, session.messages);
}

#[test]
fn unpersisted_session_still_advances_revision_and_rejects_overflow() {
    let mut session = Session {
        revision: 7,
        messages: vec![],
        next_turn: 0,
        completed: false,
        unresolved_effect: false,
    };
    assert_eq!(().save(&session).unwrap(), 8);
    session.revision = u64::MAX;
    assert!(matches!(().save(&session), Err(SessionError::Storage)));
}

impl Model for Done {
    async fn complete(
        &self,
        messages: &[Message],
        _: &[ToolSpec],
    ) -> Result<ModelReply, ModelError> {
        assert_eq!(messages[0], Message::User("work".into()));
        Ok(ModelReply {
            message: Message::text("done"),
            status: TurnStatus::Complete,
        })
    }
}

#[tokio::test]
async fn completed_history_survives_and_is_not_executed_again() {
    let store = MemorySessionStore::default();
    let agent = Agent::new(Done).session(&store);
    agent.run("work", &CancellationToken::new()).await.unwrap();
    let saved = store.load().unwrap().unwrap();
    assert!(saved.completed);
    assert_eq!(saved.messages.len(), 2);
    assert_eq!(saved.messages[1], Message::text("done"));
    assert!(agent.run("work", &CancellationToken::new()).await.is_err());
}

#[test]
fn session_store_refuses_stale_revision() {
    let store = MemorySessionStore::default();
    let mut session = Session {
        revision: 0,
        messages: vec![Message::User("work".into())],
        next_turn: 0,
        completed: false,
        unresolved_effect: false,
    };
    session.revision = store.save(&session).unwrap();
    assert_eq!(session.revision, 1);
    assert!(matches!(
        store.save(&Session {
            revision: 0,
            ..session.clone()
        }),
        Err(SessionError::Conflict)
    ));
    assert_eq!(store.load().unwrap().unwrap(), session);
}

struct FailingStore {
    inner: MemorySessionStore,
    fail_at: u64,
}

impl SessionStore for FailingStore {
    fn load(&self) -> Result<Option<Session>, SessionError> {
        self.inner.load()
    }
    fn save(&self, session: &Session) -> Result<u64, SessionError> {
        if session.revision == self.fail_at {
            return Err(SessionError::Storage);
        }
        self.inner.save(session)
    }
}

struct CallModel;

impl Model for CallModel {
    async fn complete(&self, _: &[Message], _: &[ToolSpec]) -> Result<ModelReply, ModelError> {
        Ok(ModelReply {
            message: Message::call("call", "count", serde_json::json!({})),
            status: TurnStatus::ToolCalls,
        })
    }
}

struct CountTool(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl agent_harness::Tool for CountTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "count".into(),
            description: "count".into(),
            parameters: serde_json::json!({"type":"object"}),
        }
    }
    fn call(&self, _: serde_json::Value) -> agent_harness::ToolFuture<'_> {
        Box::pin(async {
            self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok("counted".into())
        })
    }
}

#[tokio::test]
async fn failed_initial_or_tool_dispatch_history_save_prevents_side_effects() {
    for fail_at in [0, 1] {
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let store = FailingStore {
            inner: MemorySessionStore::default(),
            fail_at,
        };
        let result = Agent::new(CallModel)
            .tool(CountTool(count.clone()))
            .session(store)
            .run("work", &CancellationToken::new())
            .await;
        assert!(matches!(
            result,
            Err(agent_harness::RunError::Session(SessionError::Storage))
        ));
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn unresolved_call_after_crash_is_not_dispatched_again() {
    let store = MemorySessionStore::default();
    store
        .save(&Session {
            revision: 0,
            messages: vec![
                Message::User("work".into()),
                Message::call("call", "count", serde_json::json!({})),
            ],
            next_turn: 0,
            completed: false,
            unresolved_effect: false,
        })
        .unwrap();
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for _ in 0..2 {
        let result = Agent::new(CallModel)
            .tool(CountTool(count.clone()))
            .session(&store)
            .run("work", &CancellationToken::new())
            .await;
        assert!(matches!(
            result,
            Err(agent_harness::RunError::Session(SessionError::Unsettled))
        ));
        let saved = store.load().unwrap().unwrap();
        assert!(saved.unresolved_effect);
        assert_eq!(saved.revision, 2);
        assert_eq!(
            saved.messages[2],
            Message::Tool {
                call_id: "call".into(),
                content: "結果未知".into(),
                is_error: true,
            }
        );
    }
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[tokio::test]
async fn recovery_keeps_known_results_and_saves_unknown_once() {
    let store = MemorySessionStore::default();
    let batch = BatchModel.complete(&[], &[]).await.unwrap().message;
    let known = Message::Tool {
        call_id: "first".into(),
        content: "counted".into(),
        is_error: false,
    };
    store
        .save(&Session {
            revision: 0,
            messages: vec![Message::User("work".into()), batch, known.clone()],
            next_turn: 0,
            completed: false,
            unresolved_effect: false,
        })
        .unwrap();
    for _ in 0..2 {
        let result = Agent::new(Done)
            .session(&store)
            .run("work", &CancellationToken::new())
            .await;
        assert!(matches!(
            result,
            Err(agent_harness::RunError::Session(SessionError::Unsettled))
        ));
        let saved = store.load().unwrap().unwrap();
        assert!(saved.unresolved_effect);
        assert_eq!(saved.revision, 2);
        assert_eq!(saved.messages.len(), 4);
        assert_eq!(saved.messages[2], known);
        assert_eq!(
            saved.messages[3],
            Message::Tool {
                call_id: "second".into(),
                content: "結果未知".into(),
                is_error: true,
            }
        );
    }
}

#[tokio::test]
async fn failed_recovery_save_preserves_original_history_and_prevents_dispatch() {
    let store = FailingStore {
        inner: MemorySessionStore::default(),
        fail_at: 1,
    };
    let original = Session {
        revision: 1,
        messages: vec![
            Message::User("work".into()),
            Message::call("call", "count", serde_json::json!({})),
        ],
        next_turn: 0,
        completed: false,
        unresolved_effect: false,
    };
    store
        .inner
        .save(&Session {
            revision: 0,
            ..original.clone()
        })
        .unwrap();
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for _ in 0..2 {
        let result = Agent::new(CallModel)
            .session(&store)
            .tool(CountTool(count.clone()))
            .run("work", &CancellationToken::new())
            .await;
        assert!(matches!(
            result,
            Err(agent_harness::RunError::Session(SessionError::Storage))
        ));
        assert_eq!(store.load().unwrap().unwrap(), original);
    }
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
fn serialized_history_preserves_bad_arguments_and_error_results() {
    let mut call = Message::call("id", "tool", serde_json::json!({}));
    if let Message::Assistant { calls, .. } = &mut call {
        calls[0].bad_args = Some("{broken".into());
    }
    let messages = vec![
        Message::User("中文".into()),
        call,
        Message::Tool {
            call_id: "id".into(),
            content: "錯誤".into(),
            is_error: true,
        },
    ];
    let raw = serde_json::to_vec(&messages).unwrap();
    assert_eq!(
        serde_json::from_slice::<Vec<Message>>(&raw).unwrap(),
        messages
    );
}

struct StopAfterResult;

#[tokio::test]
async fn malformed_persisted_pairs_are_rejected_before_model_use() {
    let call = Message::call("call", "count", serde_json::json!({}));
    let result = Message::Tool {
        call_id: "call".into(),
        content: "counted".into(),
        is_error: false,
    };
    let orphan = Message::Tool {
        call_id: "other".into(),
        content: "orphan".into(),
        is_error: false,
    };
    for tail in [
        vec![result.clone()],
        vec![call.clone(), orphan],
        vec![call.clone(), call.clone(), result.clone()],
        vec![call.clone(), result.clone(), result.clone()],
        vec![result.clone(), call.clone()],
        vec![call.clone(), Message::text("continued without result")],
        vec![call.clone(), Message::User("next prompt".into())],
        vec![
            call.clone(),
            Message::call("next", "count", serde_json::json!({})),
        ],
    ] {
        let store = MemorySessionStore::default();
        let mut messages = vec![Message::User("work".into())];
        messages.extend(tail);
        let original = Session {
            revision: 1,
            messages: messages.clone(),
            next_turn: 0,
            completed: false,
            unresolved_effect: false,
        };
        store
            .save(&Session {
                revision: 0,
                messages,
                next_turn: 0,
                completed: false,
                unresolved_effect: false,
            })
            .unwrap();
        let result = Agent::new(Done)
            .session(&store)
            .run("work", &CancellationToken::new())
            .await;
        assert!(matches!(
            result,
            Err(agent_harness::RunError::Session(SessionError::Unsettled))
        ));
        assert_eq!(store.load().unwrap().unwrap(), original);
    }
}

struct BatchModel;

impl Model for BatchModel {
    async fn complete(&self, _: &[Message], _: &[ToolSpec]) -> Result<ModelReply, ModelError> {
        let mut message = Message::call("first", "count", serde_json::json!({}));
        if let Message::Assistant { calls, .. } = &mut message {
            let mut second = calls[0].clone();
            second.id = "second".into();
            calls.push(second);
        }
        Ok(ModelReply {
            message,
            status: TurnStatus::ToolCalls,
        })
    }
}

struct StopBoundary(bool);

impl agent_harness::Hook for StopBoundary {
    async fn on_dispatch(&self, _: agent_harness::Dispatch<'_>) -> agent_harness::DispatchAction {
        if self.0 {
            return agent_harness::DispatchAction::Stop("stop before dispatch".into());
        }
        agent_harness::DispatchAction::Proceed
    }

    async fn on_outcome(&self, _: agent_harness::Outcome<'_>) -> agent_harness::OutcomeAction {
        agent_harness::OutcomeAction::Stop("stop after outcome".into())
    }
}

#[tokio::test]
async fn hook_stop_settles_every_unstarted_call_and_keeps_known_result() {
    for before in [true, false] {
        let store = MemorySessionStore::default();
        let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let result = Agent::new(BatchModel)
            .session(&store)
            .hook(StopBoundary(before))
            .tool(CountTool(count.clone()))
            .run("work", &CancellationToken::new())
            .await;
        assert!(matches!(result, Err(agent_harness::RunError::Stopped(_))));
        let saved = store.load().unwrap().unwrap();
        assert_eq!(saved.messages.len(), 4);
        assert!(
            matches!(&saved.messages[3], Message::Tool { call_id, is_error: true, .. } if call_id == "second")
        );
        assert!(
            matches!(&saved.messages[2], Message::Tool { call_id, is_error, .. } if call_id == "first" && *is_error == before)
        );
        assert_eq!(
            count.load(std::sync::atomic::Ordering::SeqCst),
            usize::from(!before)
        );
        Agent::new(Done)
            .session(&store)
            .run("work", &CancellationToken::new())
            .await
            .unwrap();
    }
}

struct CancelAfterEffect {
    count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    cancellation: CancellationToken,
}

impl agent_harness::Tool for CancelAfterEffect {
    fn spec(&self) -> ToolSpec {
        CountTool(self.count.clone()).spec()
    }

    fn call(&self, _: serde_json::Value) -> agent_harness::ToolFuture<'_> {
        Box::pin(async {
            self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            self.cancellation.cancel();
            std::future::pending().await
        })
    }
}

#[tokio::test]
async fn cancelled_unknown_effect_blocks_reentry_before_model_or_tool() {
    let store = MemorySessionStore::default();
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let cancellation = CancellationToken::new();
    let result = Agent::new(CallModel)
        .session(&store)
        .tool(CancelAfterEffect {
            count: count.clone(),
            cancellation: cancellation.clone(),
        })
        .run("work", &cancellation)
        .await;
    assert!(matches!(
        result,
        Err(agent_harness::RunError::Cancelled { .. })
    ));
    let saved = store.load().unwrap().unwrap();
    assert_eq!(saved.messages.len(), 3);
    for _ in 0..2 {
        let result = Agent::new(CallModel)
            .session(&store)
            .tool(CountTool(count.clone()))
            .run("work", &CancellationToken::new())
            .await;
        assert!(matches!(
            result,
            Err(agent_harness::RunError::Session(SessionError::Unsettled))
        ));
    }
    assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
}

impl agent_harness::Hook for StopAfterResult {
    async fn on_outcome(&self, _: agent_harness::Outcome<'_>) -> agent_harness::OutcomeAction {
        agent_harness::OutcomeAction::Stop("receipt unavailable".into())
    }
}

#[tokio::test]
async fn hook_stop_preserves_the_known_tool_result() {
    let store = MemorySessionStore::default();
    let count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let result = Agent::new(CallModel)
        .session(&store)
        .hook(StopAfterResult)
        .tool(CountTool(count))
        .run("work", &CancellationToken::new())
        .await;
    assert!(matches!(result, Err(agent_harness::RunError::Stopped(_))));
    let saved = store.load().unwrap().unwrap();
    assert!(
        matches!(&saved.messages[2], Message::Tool { call_id, content, is_error: false } if call_id == "call" && content == "counted")
    );
}
