use crate::prelude::*;
use crate::settings::DATABASE_FILE;
use agent_platform_agent::{single_link::single_link, store::TaskStore};
use std::path::PathBuf;

pub fn open_store(access: StoreAccess) -> Result<TaskStore, Message> {
    let create = access == StoreAccess::Create;
    let directory = match std::env::var_os(STATE_DIR_ENV) {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        Some(_) => return Err(Message::StateDirEmpty),
        None => directories::ProjectDirs::from("", "", APP_NAME)
            .map(|dirs| dirs.data_local_dir().to_path_buf())
            .ok_or(Message::NoUserDataDir)?,
    };
    if !directory.is_absolute() {
        return Err(Message::StateDirNotAbsolute);
    }
    if create {
        crate::state_dir::create_directory(&directory)
            .map_err(|_| Message::StateDirCreateFailed)?;
    }
    let metadata = std::fs::symlink_metadata(&directory).map_err(|_| Message::StateDirMissing)?;
    if !metadata.is_dir() {
        return Err(Message::StateDirNotPlain);
    }
    if !crate::state_dir::is_private(&metadata) {
        return Err(Message::StateDirTooOpen);
    }
    let path = directory.join(DATABASE_FILE);
    match std::fs::symlink_metadata(&path) {
        Ok(metadata) => {
            if !metadata.is_file() {
                return Err(Message::DatabaseNotPlain);
            }
            if !single_link(&metadata) {
                return Err(Message::DatabaseHardLink);
            }
        }
        Err(error) if create && error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err(Message::DatabaseUnreadable),
    }
    match access {
        StoreAccess::Create => TaskStore::open(&path),
        StoreAccess::Read => TaskStore::read(&path),
        StoreAccess::Write => TaskStore::write(&path),
    }
    .map_err(Message::from)
}
