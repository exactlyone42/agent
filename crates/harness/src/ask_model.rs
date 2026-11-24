use crate::prelude::*;

pub(crate) async fn ask_model(
    model: &impl Model,
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
        reply = model.complete(messages, specs) => Ok(reply?),
    }
}
