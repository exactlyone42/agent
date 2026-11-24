#[cfg(not(any(unix, windows)))]
compile_error!("agent-platform-agent supports only unix and windows");

#[path = "task.rs"]
mod file_task;
mod prelude;
mod task_settings;
pub mod task_timeout;
pub mod tool_refusal;
mod tool_refusal_messages;

pub mod task_approval {
    pub(crate) mod fault_of;
    pub(crate) mod gate;
    pub(crate) mod task_approvals;

    pub use gate::{TaskGate, task_gate};
    pub use task_approvals::TaskApprovals;
}

mod file_check {
    pub mod input;
    pub mod verification;

    pub mod single_link {
        #[cfg_attr(unix, path = "unix.rs")]
        #[cfg_attr(windows, path = "windows.rs")]
        mod sys;

        pub use sys::single_link;
    }
}

mod runtime {
    pub mod agent_runtime;
    pub mod arbitration;
    pub mod pending_tools;
    mod run_state;
}

pub mod store {
    pub(crate) mod append;
    pub(crate) mod approval_limits;
    pub(crate) mod approval_status;
    pub(crate) mod approvals;
    pub(crate) mod close_intents;
    pub(crate) mod decision_source;
    pub(crate) mod digest;
    pub(crate) mod event_kind;
    pub(crate) mod intent_status;
    pub(crate) mod sqlite_settings;
    pub(crate) mod task_state;
    pub(crate) mod task_store;

    pub use approval_limits::{APPROVAL_LIST_LIMIT, APPROVAL_TTL_SECONDS, TOOL_CALL_LIMIT};
    pub use approval_status::ApprovalStatus;
    pub use approvals::{ApprovalMode, ApprovalRecord, Claim, PreparedIntent, TaskSpec};
    pub use sqlite_settings::EVENT_PAGE_SIZE;
    pub use task_state::TaskState;
    pub use task_store::{StoreAccess, StoreError, TaskEvent, TaskRecord, TaskStore};
}

pub mod task {
    pub use crate::file_task::{ArtifactState, FileTask, TaskError, TaskReport, run_file_task};
    pub(crate) use crate::file_task::{ReadArgs, WriteArgs};
    pub use crate::task_settings::{
        FILE_LIMIT, MANUAL_TASK_TIMEOUT, READ_INPUT_DESCRIPTION, READ_INPUT_TOOL, TASK_TIMEOUT,
        WRITE_OUTPUT_DESCRIPTION, WRITE_OUTPUT_DONE, WRITE_OUTPUT_TOOL,
    };
    pub use agent_harness::CancellationToken;
}

pub use file_check::{input, single_link, verification};
pub use runtime::{agent_runtime, arbitration, pending_tools};
