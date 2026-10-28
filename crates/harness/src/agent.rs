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
        for turn in 0..self.max_turns {
            let reply = self.ask(&messages, &specs, cancel).await?;
            let calls = reply.calls().to_vec();
            messages.push(reply);
            if calls.is_empty() {
                return Ok(finish(messages));
            }
            for call in &calls {
                let result = self.handle(turn, call, cancel).await?;
                messages.push(result);
            }
        }
        Err(RunError::TurnLimit)
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
        let outcome = tool.call(call.args.clone()).await;
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

fn finish(messages: Vec<Message>) -> RunOutput {
    let text = match messages.last() {
        Some(Message::Assistant { text, .. }) => text.clone(),
        _ => String::new(),
    };
    RunOutput { messages, text }
}
