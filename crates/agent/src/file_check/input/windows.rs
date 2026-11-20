use std::{
    fs::{File, Metadata, OpenOptions},
    io,
    os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
    path::Path,
};
use windows_sys::Win32::Storage::FileSystem::{
    FILE_FLAG_BACKUP_SEMANTICS, FILE_ID_INFO, FILE_READ_ATTRIBUTES, FILE_SHARE_READ, FileIdInfo,
    GetFileInformationByHandleEx,
};

const FILE_ID_BYTES: usize = 16;

type FileId = (u64, [u8; FILE_ID_BYTES]);

pub(super) struct Pinned {
    _guard: File,
    id: FileId,
}

impl Pinned {
    pub(super) fn same_file(&self, file: &File) -> io::Result<bool> {
        Ok(file_identity(file)? == self.id)
    }
}

pub(super) fn pin(path: &Path) -> io::Result<(Pinned, Metadata)> {
    let guard = OpenOptions::new()
        .access_mode(FILE_READ_ATTRIBUTES)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)?;
    let metadata = guard.metadata()?;
    let id = file_identity(&guard)?;
    Ok((Pinned { _guard: guard, id }, metadata))
}

pub(super) fn read_options() -> OpenOptions {
    let mut options = OpenOptions::new();
    options.read(true).share_mode(FILE_SHARE_READ);
    options
}

fn file_identity(file: &File) -> io::Result<FileId> {
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

#[cfg(test)]
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
