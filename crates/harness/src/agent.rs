use crate::finish::finish;
use crate::{
    Dispatch, DispatchAction, Hook, Message, Model, ModelError, ModelReply, Outcome, OutcomeAction,
    Tool, ToolCall, ToolResult, ToolSpec, TurnStatus,
};
use std::collections::HashSet;
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
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

pub struct Agent<M, H = ()> {
    model: M,
    hook: H,
    tools: Vec<Box<dyn Tool>>,
    max_turns: usize,
}

impl<M: Model> Agent<M> {
    pub fn new(model: M) -> Self {
        Self {
            model,
            hook: (),
            tools: Vec::new(),
            max_turns: 8,
        }
    }
}

impl<M: Model, H: Hook> Agent<M, H> {
    pub fn hook<N: Hook>(self, hook: N) -> Agent<M, N> {
        Agent {
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
        let specs: Vec<ToolSpec> = self.tools.iter().map(|t| t.spec()).collect();
        let mut messages = vec![Message::User(prompt.to_owned())];
        let mut turn = 0;
        let mut finished = None;
        while finished.is_none() && turn < self.max_turns {
            finished = self.step(turn, &mut messages, &specs, cancel).await?;
            turn += 1;
        }
        finished.ok_or(RunError::TurnLimit)
    }

    async fn step(
        &self,
        turn: usize,
        messages: &mut Vec<Message>,
        specs: &[ToolSpec],
        cancel: &CancellationToken,
    ) -> Result<Option<RunOutput>, RunError> {
        let reply = self
            .ask(messages, specs, cancel)
            .await
            .map_err(|error| Self::attach_history(error, messages))?;
        let calls = reply.message.calls().to_vec();
        if reply.status == TurnStatus::ToolCalls {
            Self::validate_calls(messages, &calls)?;
        }
        messages.push(reply.message);
        match reply.status {
            TurnStatus::Complete if calls.is_empty() => {
                return Ok(Some(finish(std::mem::take(messages))));
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
        self.settle_calls(turn, messages, &calls, cancel).await?;
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
        turn: usize,
        messages: &mut Vec<Message>,
        calls: &[ToolCall],
        cancel: &CancellationToken,
    ) -> Result<(), RunError> {
        for (index, call) in calls.iter().enumerate() {
            let result = self.answer(turn, call, cancel).await;
            Self::settle_result(result, messages, &calls[index..])?;
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
            Err(CallError::Stopped(why)) => return Err(RunError::Stopped(why)),
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

    async fn answer(
        &self,
        turn: usize,
        call: &ToolCall,
        cancel: &CancellationToken,
    ) -> Result<Message, CallError> {
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
    ) -> Result<Message, CallError> {
        let tool_message = |content: String, is_error: bool| Message::Tool {
            call_id: call.id.clone(),
            content,
            is_error,
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
        let hook_result = tokio::select! {
            biased;
            action = self.hook.on_outcome(Outcome { turn, call, result }) => action,
            _ = cancel.cancelled() => OutcomeAction::Proceed,
        };
        if let OutcomeAction::Stop(why) = hook_result {
            return Err(CallError::Stopped(why));
        }
        Ok(match outcome {
            Ok(content) => tool_message(content, false),
            Err(error) => tool_message(error.to_string(), true),
        })
    }
}
