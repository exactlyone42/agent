use std::{
    fs::{File, OpenOptions},
    io,
    path::Path,
};

#[derive(Debug, thiserror::Error)]
pub enum InputError {
    #[error("檔案類型、大小或身分不符合要求")]
    InvalidFile,
    #[error("作業系統無法開啟或核對檔案")]
    Io(#[from] io::Error),
}

pub fn open_checked(path: &Path, limit: u64) -> Result<File, InputError> {
    open_file(path, limit, false)
}

pub fn open_result(path: &Path, limit: u64) -> Result<File, InputError> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() {
        return Err(InputError::InvalidFile);
    }
    open_file(path, limit, true)
}

fn open_file(path: &Path, limit: u64, no_follow: bool) -> Result<File, InputError> {
    #[cfg(unix)]
    let before = std::fs::metadata(path)?;
    #[cfg(windows)]
    let identity_guard = {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::{
            FILE_FLAG_BACKUP_SEMANTICS, FILE_READ_ATTRIBUTES, FILE_SHARE_READ,
        };
        let guard = OpenOptions::new()
            .access_mode(FILE_READ_ATTRIBUTES)
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
            .open(path)?;
        let metadata = guard.metadata()?;
        if !metadata.is_file() || metadata.len() > limit {
            return Err(InputError::InvalidFile);
        }
        guard
    };
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let flags = libc::O_NONBLOCK | libc::O_NOCTTY;
        options.custom_flags(if no_follow {
            flags | libc::O_NOFOLLOW
        } else {
            flags
        });
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
        options.share_mode(FILE_SHARE_READ);
        if no_follow {
            use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
            options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        }
    }
    let file = options.open(path)?;
    let after = file.metadata()?;
    if !after.is_file() || after.len() > limit {
        return Err(InputError::InvalidFile);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if before.dev() != after.dev() || before.ino() != after.ino() {
            return Err(InputError::InvalidFile);
        }
    }
    #[cfg(windows)]
    if file_identity(&identity_guard)? != file_identity(&file)? {
        return Err(InputError::InvalidFile);
    }
    Ok(file)
}

#[cfg(windows)]
fn file_identity(file: &File) -> io::Result<(u64, [u8; 16])> {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ID_INFO, FileIdInfo, GetFileInformationByHandleEx,
    };
    let mut info = FILE_ID_INFO::default();
    let result = unsafe {
        GetFileInformationByHandleEx(
            file.as_raw_handle(),
            FileIdInfo,
            std::ptr::from_mut(&mut info).cast(),
            std::mem::size_of::<FILE_ID_INFO>() as u32,
        )
    };
    if result == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok((info.VolumeSerialNumber, info.FileId.Identifier))
}

#[cfg(all(test, windows))]
mod tests {
    use super::file_identity;
    use std::fs::File;

    #[test]
    fn identity_distinguishes_files_and_recognizes_hard_links()
    -> Result<(), Box<dyn std::error::Error>> {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let root = std::env::temp_dir().join(format!("agent-id-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&root)?;
        let source = root.join("source");
        let other = root.join("other");
        let alias = root.join("alias");
        std::fs::write(&source, "same content")?;
        std::fs::write(&other, "same content")?;
        std::fs::hard_link(&source, &alias)?;
        {
            let source = File::open(&source)?;
            let other = File::open(&other)?;
            let alias = File::open(&alias)?;
            assert_ne!(file_identity(&source)?, file_identity(&other)?);
            assert_eq!(file_identity(&source)?, file_identity(&alias)?);
        }
        std::fs::remove_file(source)?;
        std::fs::remove_file(other)?;
        std::fs::remove_file(alias)?;
        std::fs::remove_dir(root)?;
        Ok(())
    }
}

#[cfg(all(test, unix))]
mod result_tests {
    #[test]
    fn result_open_refuses_link_even_after_precheck_is_bypassed()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("result-link-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root)?;
        let source = root.join("source");
        let alias = root.join("alias");
        std::fs::write(&source, "private")?;
        std::os::unix::fs::symlink(&source, &alias)?;
        assert!(
            super::open_file(&alias, 1024, true).is_err(),
            "opening must refuse the replaced symlink itself"
        );
        assert!(super::open_checked(&alias, 1024).is_ok());
        Ok(())
    }
}
