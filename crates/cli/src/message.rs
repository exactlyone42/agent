use crate::prelude::*;
use crate::text::{en_text::en_text, zh_tw_text::zh_tw_text};

#[derive(Debug)]
pub enum Message {
    Help,
    StateDirEmpty,
    NoUserDataDir,
    StateDirNotAbsolute,
    StateDirCreateFailed,
    StateDirMissing,
    StateDirNotPlain,
    StateDirTooOpen,
    DatabaseNotPlain,
    DatabaseHardLink,
    DatabaseUnreadable,
    InvalidTaskId,
    InvalidApprovalId,
    InvalidRevision,
    InvalidEventPosition,
    OutputFailed,
    TaskFileOpenFailed,
    TaskFileReadFailed,
    TaskFileTooLarge,
    TaskFileInvalid,
    EndpointRejected,
    ApiKeyInvalid,
    ModelClientFailed,
    InterruptHandlerFailed,
    WorkingDirUnavailable,
    TaskRecordPoisoned,
    OutcomeUnknown,
    StatusMayBeStale,
    TaskStarted(String),
    ManualModeHint,
    TaskCancelled,
    Store(StoreError),
    Task(TaskError),
    FinishUnconfirmed(StoreError),
}

impl Message {
    pub fn text(&self, locale: Locale) -> String {
        match locale {
            Locale::ZhTw => zh_tw_text(self),
            Locale::En => en_text(self),
        }
    }
}

impl From<StoreError> for Message {
    fn from(error: StoreError) -> Self {
        Self::Store(error)
    }
}

impl From<TaskError> for Message {
    fn from(error: TaskError) -> Self {
        Self::Task(error)
    }
}
