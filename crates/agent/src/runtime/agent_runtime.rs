use crate::prelude::*;
use crate::runtime::run_state::RunState;
use agent_harness::{Capability, Dispatch, DispatchAction, Outcome, OutcomeAction, ToolResult};

pub struct AgentRuntime<M, H = ()> {
    agent: MortalAgent,
    model: M,
    hook: H,
    tools: Vec<Box<dyn Tool>>,
    capabilities: Vec<Box<dyn Capability>>,
}

impl<M: Model> AgentRuntime<M> {
    pub fn new(agent: MortalAgent, model: M) -> Self {
        Self {
            agent,
            model,
            hook: (),
            tools: Vec::new(),
            capabilities: Vec::new(),
        }
    }
}

impl<M: Model, H: Hook> AgentRuntime<M, H> {
    pub fn hook<N: Hook>(self, hook: N) -> AgentRuntime<M, N> {
        AgentRuntime {
            agent: self.agent,
            model: self.model,
            hook,
            tools: self.tools,
            capabilities: self.capabilities,
        }
    }

    pub fn tool(mut self, tool: impl Tool + 'static) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    pub fn capability(mut self, capability: impl Capability + 'static) -> Self {
        self.capabilities.push(Box::new(capability));
        self
    }

    pub async fn run(
        &mut self,
        prompt: &str,
        cancel: &CancellationToken,
    ) -> Result<RunOutput, RunError> {
        let specs: Vec<ToolSpec> = self.tools.iter().map(|tool| tool.spec()).collect();
        let mut state = RunState::new(prompt);
        while let Some(event) = state.queue.pop_front() {
            self.handle(&event, &mut state, &specs, cancel).await?;
        }
        state.finish()
    }

    async fn handle(
        &mut self,
        event: &Event,
        state: &mut RunState,
        specs: &[ToolSpec],
        cancel: &CancellationToken,
    ) -> Result<(), RunError> {
        if cancel.is_cancelled() {
            return Err(RunError::Cancelled);
        }
        if !state.accepts(event) {
            return Ok(());
        }
        let context = state.context();
        let proposals = self
            .capabilities
            .iter_mut()
            .map(|capability| {
                (
                    capability.id().to_owned(),
                    capability.on_event(event, &context),
                )
            })
            .collect();
        let arbitration = Arbitration::decide_by_capability(proposals);
        state.adopt(&arbitration);
        self.run_calls(&arbitration.calls, state, cancel).await?;
        self.ask_if_ready(state, specs, cancel).await
    }

    async fn run_calls(
        &self,
        calls: &[ToolCall],
        state: &mut RunState,
        cancel: &CancellationToken,
    ) -> Result<(), RunError> {
        let accepted = state.expect_calls(calls);
        for (call, accepted) in calls.iter().zip(accepted) {
            self.run_call(call, accepted, state, cancel).await?;
        }
        Ok(())
    }

    async fn run_call(
        &self,
        call: &ToolCall,
        accepted: bool,
        state: &mut RunState,
        cancel: &CancellationToken,
    ) -> Result<(), RunError> {
        if !accepted {
            state.refuse_duplicate(call);
            return Ok(());
        }
        let turn = state.turn.saturating_sub(1);
        let (content, is_error) = self.execute(turn, call, cancel).await?;
        state.record_report(&call.id, content, is_error);
        Ok(())
    }

    async fn execute(
        &self,
        turn: usize,
        call: &ToolCall,
        cancel: &CancellationToken,
    ) -> Result<(String, bool), RunError> {
        let Some(tool) = self.tools.iter().find(|tool| tool.spec().name == call.name) else {
            return Ok((format!("there is no tool named {}", call.name), true));
        };
        if let Some(why) = self
            .capabilities
            .iter()
            .find_map(|capability| capability.forbids(call))
        {
            return Ok((why, true));
        }
        match self.hook.on_dispatch(Dispatch { turn, call }).await {
            DispatchAction::Stop(why) => return Err(RunError::Stopped(why)),
            DispatchAction::Skip(why) => return Ok((why, true)),
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
            Ok(content) => (content, false),
            Err(error) => (error.to_string(), true),
        })
    }

    async fn ask_if_ready(
        &self,
        state: &mut RunState,
        specs: &[ToolSpec],
        cancel: &CancellationToken,
    ) -> Result<(), RunError> {
        if !state.ready_to_ask() {
            return Ok(());
        }
        state.start_model_call();
        let reply = self
            .agent
            .think(&self.model, &state.work, specs, cancel)
            .await?;
        state.record_reply(reply);
        Ok(())
    }
}
