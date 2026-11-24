use crate::prelude::*;
use crate::{
    output::Cancelled, output::Finished, request::Request, settings::INTERRUPTED_EXIT_CODE,
    settings::MODEL_REQUEST_TIMEOUT, settings::REVIEW_NEEDED_EXIT_CODE,
};
use agent_harness_openai::OpenAi;
use agent_platform_agent::{
    store::TaskSpec,
    task::{CancellationToken, run_file_task},
    task_approval::task_gate,
    task_timeout::task_timeout,
};
use std::sync::{Arc, Mutex};

pub async fn run_command(locale: Locale, path: &Path) -> Result<ExitCode, Message> {
    let mut request = Request::load(path)?;
    let key = match std::env::var(API_KEY_ENV) {
        Ok(value) => value,
        Err(std::env::VarError::NotPresent) => String::new(),
        Err(_) => return Err(Message::ApiKeyInvalid),
    };
    let model = OpenAi::new(
        &request.endpoint,
        request.model.clone(),
        key,
        MODEL_REQUEST_TIMEOUT,
    )
    .map_err(|_| Message::ModelClientFailed)?;
    let cancellation = CancellationToken::new();
    let signal_cancellation = cancellation.clone();
    let mut interrupted = false;
    ctrlc::try_set_handler(move || {
        if interrupted {
            std::process::exit(i32::from(INTERRUPTED_EXIT_CODE));
        }
        interrupted = true;
        signal_cancellation.cancel();
    })
    .map_err(|_| Message::InterruptHandlerFailed)?;
    let workspace = std::env::current_dir().map_err(|_| Message::WorkingDirUnavailable)?;
    request.task.input = workspace.join(&request.task.input);
    request.task.output = workspace.join(&request.task.output);
    let spec = TaskSpec {
        workspace,
        endpoint: request.endpoint.to_string(),
        model: request.model.clone(),
        task: request.task.clone(),
        approval_mode: request.approval_mode,
    };
    let mut store = open_store(StoreAccess::Create)?;
    let (task_id, generation) = store.start_spec(&spec)?;
    let store = Arc::new(Mutex::new(store));
    let gate = task_gate(
        store.clone(),
        task_id.clone(),
        cancellation.clone(),
        request.approval_mode == ApprovalMode::Manual,
    );
    eprintln!("{}", Message::TaskStarted(task_id.clone()).text(locale));
    if request.approval_mode == ApprovalMode::Manual {
        eprintln!("{}", Message::ManualModeHint.text(locale));
    }
    let mut result = run_file_task(
        model,
        request.task,
        cancellation,
        gate.clone(),
        task_timeout(request.approval_mode),
    )
    .await;
    if matches!(result, Err(TaskError::Harness(_))) && gate.was_denied() {
        result = Err(TaskError::ApprovalDenied);
    }
    let mut store = store.lock().map_err(|_| Message::TaskRecordPoisoned)?;
    store
        .finish(&task_id, generation, &result)
        .map_err(Message::FinishUnconfirmed)?;
    let recorded_state = store.status(&task_id)?.recorded_state;
    let report = match result {
        Err(TaskError::Cancelled { artifact_created }) => {
            println!(
                "{}",
                serde_json::to_string(&Cancelled {
                    artifact_created,
                    state: &recorded_state,
                    task_id: &task_id,
                })
                .map_err(|_| Message::OutputFailed)?
            );
            eprintln!("{}", Message::TaskCancelled.text(locale));
            return Ok(ExitCode::from(INTERRUPTED_EXIT_CODE));
        }
        other => other?,
    };
    if recorded_state == TaskState::OutcomeUnknown.as_str() {
        return Err(Message::OutcomeUnknown);
    }
    println!(
        "{}",
        serde_json::to_string(&Finished {
            artifact: &report.artifact,
            assistant_message: &report.assistant_message,
            task_id: &task_id,
        })
        .map_err(|_| Message::OutputFailed)?
    );
    Ok(if report.artifact == ArtifactState::Verified {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(REVIEW_NEEDED_EXIT_CODE)
    })
}
