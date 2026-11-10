use crate::finish::finish;
use crate::{
    Dispatch, DispatchAction, Hook, Message, Model, ModelError, ModelReply, Outcome, OutcomeAction,
    Session, SessionError, SessionStore, Tool, ToolCall, ToolResult, ToolSpec, TurnStatus,
};
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("{0}")]
    Session(#[from] SessionError),
    #[error("已要求停止")]
    Cancelled { messages: Vec<Message> },
    #[error("超過回合上限")]
    TurnLimit,
    #[error("模型輸出被截斷")]
    Truncated,
    #[error("模型政策攔截")]
    Blocked,
    #[error("模型失敗：{0}")]
    Model(#[from] ModelError),
    #[error("被攔截而停止：{0}")]
    Stopped(String),
}

#[derive(Debug)]
pub struct RunOutput {
    pub messages: Vec<Message>,
    pub text: String,
}

enum CallError {
    Cancelled { dispatched: bool },
    Stopped(String),
}

pub struct Agent<M, H = (), S = ()> {
    session: S,
    model: M,
    hook: H,
    tools: Vec<Box<dyn Tool>>,
    max_turns: usize,
}

impl<M: Model> Agent<M> {
    pub fn new(model: M) -> Self {
        Self {
            session: (),
            model,
            hook: (),
            tools: Vec::new(),
            max_turns: 8,
        }
    }
}

impl<M: Model, H: Hook, S: SessionStore> Agent<M, H, S> {
    pub fn session<N: SessionStore>(self, session: N) -> Agent<M, H, N> {
        Agent {
            model: self.model,
            hook: self.hook,
            tools: self.tools,
            max_turns: self.max_turns,
            session,
        }
    }

    pub fn hook<N: Hook>(self, hook: N) -> Agent<M, N, S> {
        Agent {
            session: self.session,
            model: self.model,
            hook,
            tools: self.tools,
            max_turns: self.max_turns,
        }
    }

    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    pub fn max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }

    pub async fn run(
        &self,
        prompt: &str,
        cancel: &CancellationToken,
    ) -> Result<RunOutput, RunError> {
        let mut session = self.session.load()?.unwrap_or_else(|| Session {
            revision: 0,
            messages: vec![Message::User(prompt.to_owned())],
            next_turn: 0,
            completed: false,
            unresolved_effect: false,
        });
        if session.completed || session.messages.first() != Some(&Message::User(prompt.to_owned()))
        {
            return Err(SessionError::Conflict.into());
        }
        if session.unresolved_effect {
            return Err(SessionError::Unsettled.into());
        }
        let missing = crate::pending_calls(&session.messages)?;
        if !missing.is_empty() {
            session
                .messages
                .extend(missing.into_iter().map(|call| Message::Tool {
                    call_id: call.id,
                    content: "結果未知".into(),
                    is_error: true,
                }));
            session.unresolved_effect = true;
            self.persist(&mut session)?;
            return Err(SessionError::Unsettled.into());
        }
        self.persist(&mut session)?;
        let specs: Vec<ToolSpec> = self.tools.iter().map(|t| t.spec()).collect();
        let mut remaining_turns = session.next_turn..self.max_turns;
        let mut finished = None;
        while finished.is_none() && remaining_turns.next().is_some() {
            finished = self.advance(&mut session, &specs, cancel).await?;
        }
        finished.ok_or(RunError::TurnLimit)
    }

    async fn advance(
        &self,
        session: &mut Session,
        specs: &[ToolSpec],
        cancel: &CancellationToken,
    ) -> Result<Option<RunOutput>, RunError> {
        self.inject(session, cancel)?;
        let result = self.step(session, specs, cancel).await;
        if let Err(RunError::Cancelled { messages }) = &result {
            session.messages = messages.clone();
        }
        if matches!(result, Ok(None)) {
            session.next_turn += 1;
        }
        self.persist(session)?;
        result
    }

    fn inject(&self, session: &mut Session, cancel: &CancellationToken) -> Result<(), RunError> {
        if cancel.is_cancelled() {
            return Ok(());
        }
        let inputs = self.session.inputs(&session.messages)?;
        if inputs.is_empty() {
            return Ok(());
        }
        session
            .messages
            .extend(inputs.into_iter().map(Message::User));
        self.persist(session)
    }

    fn persist(&self, session: &mut Session) -> Result<(), RunError> {
        session.revision = self.session.save(session)?;
        Ok(())
    }

    async fn step(
        &self,
        session: &mut Session,
        specs: &[ToolSpec],
        cancel: &CancellationToken,
    ) -> Result<Option<RunOutput>, RunError> {
        let messages = &mut session.messages;
        let reply = self
            .ask(messages, specs, cancel)
            .await
            .map_err(|error| Self::attach_history(error, messages))?;
        let calls = reply.message.calls().to_vec();
        if reply.status == TurnStatus::ToolCalls {
            Self::validate_calls(messages, &calls)?;
        }
        messages.push(reply.message);
        self.persist(session)?;
        match reply.status {
            TurnStatus::Complete if calls.is_empty() => {
                session.completed = true;
                return Ok(Some(finish(session.messages.clone())));
            }
            TurnStatus::Continue if calls.is_empty() => return Ok(None),
            TurnStatus::Truncated => return Err(RunError::Truncated),
            TurnStatus::Blocked => return Err(RunError::Blocked),
            TurnStatus::ToolCalls if !calls.is_empty() => {}
            _ => {
                return Err(RunError::Model(ModelError(
                    "模型停止原因與工具呼叫不一致".into(),
                )));
            }
        }
        self.settle_calls(session, &calls, cancel).await?;
        Ok(None)
    }

    fn attach_history(error: RunError, messages: &mut Vec<Message>) -> RunError {
        match error {
            RunError::Cancelled { .. } => RunError::Cancelled {
                messages: std::mem::take(messages),
            },
            other => other,
        }
    }

    fn validate_calls(messages: &[Message], calls: &[ToolCall]) -> Result<(), RunError> {
        let mut seen: HashSet<&str> = messages
            .iter()
            .flat_map(Message::calls)
            .map(|call| call.id.as_str())
            .collect();
        if calls.iter().any(|call| !seen.insert(call.id.as_str())) {
            return Err(RunError::Model(ModelError(
                "模型重複使用工具呼叫編號".into(),
            )));
        }
        Ok(())
    }

    async fn settle_calls(
        &self,
        session: &mut Session,
        calls: &[ToolCall],
        cancel: &CancellationToken,
    ) -> Result<(), RunError> {
        for (index, call) in calls.iter().enumerate() {
            let result = self.answer(session.next_turn, call, cancel).await;
            session.unresolved_effect |=
                matches!(result, Err(CallError::Cancelled { dispatched: true }));
            let executed = result.as_ref().ok().and_then(|(_, result)| *result);
            Self::settle_result(
                result.map(|(message, _)| message),
                &mut session.messages,
                &calls[index..],
            )?;
            self.persist(session)?;
            let action = self
                .notify_outcome(session.next_turn, call, executed, cancel)
                .await;
            Self::check_outcome(action, &mut session.messages, &calls[index + 1..], cancel)?;
        }
        Ok(())
    }

    fn settle_result(
        result: Result<Message, CallError>,
        messages: &mut Vec<Message>,
        remaining: &[ToolCall],
    ) -> Result<(), RunError> {
        match result {
            Ok(result) => messages.push(result),
            Err(CallError::Stopped(why)) => {
                Self::settle_unstarted(messages, remaining);
                return Err(RunError::Stopped(why));
            }
            Err(CallError::Cancelled { dispatched }) => {
                return Err(Self::settle_cancelled(messages, remaining, dispatched));
            }
        }
        Ok(())
    }

    fn settle_cancelled(
        messages: &mut Vec<Message>,
        remaining: &[ToolCall],
        dispatched: bool,
    ) -> RunError {
        for (offset, call) in remaining.iter().enumerate() {
            let content = match (offset, dispatched) {
                (0, true) => "結果未知",
                _ => "未執行",
            };
            messages.push(Message::Tool {
                call_id: call.id.clone(),
                content: content.into(),
                is_error: true,
            });
        }
        RunError::Cancelled {
            messages: std::mem::take(messages),
        }
    }

    fn settle_unstarted(messages: &mut Vec<Message>, remaining: &[ToolCall]) {
        for call in remaining {
            messages.push(Message::Tool {
                call_id: call.id.clone(),
                content: "未執行".into(),
                is_error: true,
            });
        }
    }

    async fn answer(
        &self,
        turn: usize,
        call: &ToolCall,
        cancel: &CancellationToken,
    ) -> Result<(Message, Option<ToolResult>), CallError> {
        if cancel.is_cancelled() {
            return Err(CallError::Cancelled { dispatched: false });
        }
        self.handle(turn, call, cancel).await
    }

    async fn ask(
        &self,
        messages: &[Message],
        specs: &[ToolSpec],
        cancel: &CancellationToken,
    ) -> Result<ModelReply, RunError> {
        if cancel.is_cancelled() {
            return Err(RunError::Cancelled {
                messages: Vec::new(),
            });
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(RunError::Cancelled { messages: Vec::new() }),
            reply = self.model.complete(messages, specs) => Ok(reply?),
        }
    }

    async fn handle(
        &self,
        turn: usize,
        call: &ToolCall,
        cancel: &CancellationToken,
    ) -> Result<(Message, Option<ToolResult>), CallError> {
        let tool_message = |content: String, is_error: bool| {
            (
                Message::Tool {
                    call_id: call.id.clone(),
                    content,
                    is_error,
                },
                None,
            )
        };
        if call.bad_args.is_some() {
            return Ok(tool_message("工具參數不是有效 JSON".into(), true));
        }
        let Some(tool) = self.tools.iter().find(|t| t.spec().name == call.name) else {
            return Ok(tool_message(format!("沒有名為 {} 的工具", call.name), true));
        };
        let action = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(CallError::Cancelled { dispatched: false }),
            action = self.hook.on_dispatch(Dispatch { turn, call }) => action,
        };
        if cancel.is_cancelled() {
            return Err(CallError::Cancelled { dispatched: false });
        }
        match action {
            DispatchAction::Stop(why) => return Err(CallError::Stopped(why)),
            DispatchAction::Skip(why) => return Ok(tool_message(why, true)),
            DispatchAction::Proceed => {}
        }
        if cancel.is_cancelled() {
            return Err(CallError::Cancelled { dispatched: false });
        }
        let outcome = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(CallError::Cancelled { dispatched: true }),
            outcome = tool.call(call.args.clone()) => outcome,
        };
        let result = if outcome.is_ok() {
            ToolResult::Succeeded
        } else {
            ToolResult::Failed
        };
        let message = match outcome {
            Ok(content) => tool_message(content, false).0,
            Err(error) => tool_message(error.to_string(), true).0,
        };
        Ok((message, Some(result)))
    }

    async fn notify_outcome(
        &self,
        turn: usize,
        call: &ToolCall,
        result: Option<ToolResult>,
        cancel: &CancellationToken,
    ) -> OutcomeAction {
        let Some(result) = result else {
            return OutcomeAction::Proceed;
        };
        tokio::select! {
            biased;
            _ = cancel.cancelled() => OutcomeAction::Proceed,
            action = self.hook.on_outcome(Outcome { turn, call, result }) => action,
        }
    }

    fn check_outcome(
        action: OutcomeAction,
        messages: &mut Vec<Message>,
        remaining: &[ToolCall],
        cancel: &CancellationToken,
    ) -> Result<(), RunError> {
        if matches!(action, OutcomeAction::Stop(_)) && cancel.is_cancelled() {
            return Err(Self::settle_cancelled(messages, remaining, false));
        }
        if let OutcomeAction::Stop(why) = action {
            Self::settle_unstarted(messages, remaining);
            return Err(RunError::Stopped(why));
        }
        Ok(())
    }
}
