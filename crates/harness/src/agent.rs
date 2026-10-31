use crate::finish::finish;
use crate::{
    Dispatch, DispatchAction, Hook, Message, Model, ModelError, Outcome, OutcomeAction, Tool,
    ToolCall, ToolResult, ToolSpec,
};
use tokio_util::sync::CancellationToken;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("已要求停止")]
    Cancelled,
    #[error("超過回合上限")]
    TurnLimit,
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
        let reply = self.ask(messages, specs, cancel).await?;
        let calls = reply.calls().to_vec();
        messages.push(reply);
        if calls.is_empty() {
            return Ok(Some(finish(std::mem::take(messages))));
        }
        for (index, call) in calls.iter().enumerate() {
            let result = self.answer(turn, &calls[..index], call, cancel).await?;
            messages.push(result);
        }
        Ok(None)
    }

    async fn answer(
        &self,
        turn: usize,
        earlier: &[ToolCall],
        call: &ToolCall,
        cancel: &CancellationToken,
    ) -> Result<Message, RunError> {
        if earlier.iter().any(|other| other.id == call.id) {
            return Ok(Message::Tool {
                call_id: call.id.clone(),
                content: "呼叫編號重複，未執行".into(),
                is_error: true,
            });
        }
        self.handle(turn, call, cancel).await
    }

    async fn ask(
        &self,
        messages: &[Message],
        specs: &[ToolSpec],
        cancel: &CancellationToken,
    ) -> Result<Message, RunError> {
        if cancel.is_cancelled() {
            return Err(RunError::Cancelled);
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(RunError::Cancelled),
            reply = self.model.complete(messages, specs) => Ok(reply?),
        }
    }

    async fn handle(
        &self,
        turn: usize,
        call: &ToolCall,
        cancel: &CancellationToken,
    ) -> Result<Message, RunError> {
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
        match self.hook.on_dispatch(Dispatch { turn, call }).await {
            DispatchAction::Stop(why) => return Err(RunError::Stopped(why)),
            DispatchAction::Skip(why) => return Ok(tool_message(why, true)),
            DispatchAction::Proceed => {}
        }
        if cancel.is_cancelled() {
            return Err(RunError::Cancelled);
        }
        let outcome = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(RunError::Cancelled),
            outcome = tool.call(call.args.clone()) => outcome,
        };
        let result = if outcome.is_ok() {
            ToolResult::Succeeded
        } else {
            ToolResult::Failed
        };
        if let OutcomeAction::Stop(why) = self.hook.on_outcome(Outcome { turn, call, result }).await
        {
            return Err(RunError::Stopped(why));
        }
        Ok(match outcome {
            Ok(content) => tool_message(content, false),
            Err(error) => tool_message(error.to_string(), true),
        })
    }
}
