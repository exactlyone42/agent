use crate::prelude::*;
use crate::{settings::SCHEME_HTTP, settings::SCHEME_HTTPS, settings::TASK_FILE_LIMIT};
use agent_platform_agent::{input::open_checked, store::ApprovalMode, task::FileTask};
use serde::Deserialize;
use std::{io::Read, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub endpoint: url::Url,
    pub model: String,
    pub task: FileTask,
    #[serde(default)]
    pub approval_mode: ApprovalMode,
}

impl Request {
    pub fn load(path: &Path) -> Result<Self, Message> {
        let mut bytes = Vec::new();
        open_checked(path, u64::MAX)
            .map_err(|_| Message::TaskFileOpenFailed)?
            .take(TASK_FILE_LIMIT + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Message::TaskFileReadFailed)?;
        if bytes.len() as u64 > TASK_FILE_LIMIT {
            return Err(Message::TaskFileTooLarge);
        }
        let request: Self = serde_json::from_slice(&bytes).map_err(|_| Message::TaskFileInvalid)?;
        request.check_endpoint()?;
        Ok(request)
    }

    fn check_endpoint(&self) -> Result<(), Message> {
        let loopback = match self.endpoint.host() {
            Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
            Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
            _ => false,
        };
        if !loopback
            || !matches!(self.endpoint.scheme(), SCHEME_HTTP | SCHEME_HTTPS)
            || !self.endpoint.username().is_empty()
            || self.endpoint.password().is_some()
            || self.endpoint.query().is_some()
            || self.endpoint.fragment().is_some()
            || self.model.trim().is_empty()
        {
            return Err(Message::EndpointRejected);
        }
        Ok(())
    }
}
