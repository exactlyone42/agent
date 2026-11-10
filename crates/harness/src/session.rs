use crate::Message;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub revision: u64,
    pub messages: Vec<Message>,
    pub next_turn: usize,
    pub completed: bool,
    pub unresolved_effect: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("對話紀錄已變更，不可用舊版本繼續")]
    Conflict,
    #[error("無法保存或讀取對話紀錄")]
    Storage,
    #[error("對話紀錄尚有未確認結果，不可自動接續")]
    Unsettled,
}

pub trait SessionStore: Send + Sync {
    fn load(&self) -> Result<Option<Session>, SessionError>;
    fn save(&self, session: &Session) -> Result<u64, SessionError>;

    fn inputs(&self, _: &[Message]) -> Result<Vec<String>, SessionError> {
        Ok(Vec::new())
    }
}

impl<T: SessionStore + ?Sized> SessionStore for &T {
    fn inputs(&self, messages: &[Message]) -> Result<Vec<String>, SessionError> {
        (**self).inputs(messages)
    }

    fn load(&self) -> Result<Option<Session>, SessionError> {
        (**self).load()
    }

    fn save(&self, session: &Session) -> Result<u64, SessionError> {
        (**self).save(session)
    }
}

impl SessionStore for () {
    fn load(&self) -> Result<Option<Session>, SessionError> {
        Ok(None)
    }

    fn save(&self, session: &Session) -> Result<u64, SessionError> {
        session.revision.checked_add(1).ok_or(SessionError::Storage)
    }
}

#[derive(Default)]
pub struct MemorySessionStore(Mutex<Option<Session>>);

impl SessionStore for MemorySessionStore {
    fn load(&self) -> Result<Option<Session>, SessionError> {
        Ok(self.0.lock().map_err(|_| SessionError::Storage)?.clone())
    }

    fn save(&self, session: &Session) -> Result<u64, SessionError> {
        let mut current = self.0.lock().map_err(|_| SessionError::Storage)?;
        if current.as_ref().map_or(0, |s| s.revision) != session.revision {
            return Err(SessionError::Conflict);
        }
        let revision = session
            .revision
            .checked_add(1)
            .ok_or(SessionError::Storage)?;
        *current = Some(Session {
            revision,
            ..session.clone()
        });
        Ok(revision)
    }
}
