use std::ffi::CString;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub fn make_fifo(path: &Path) {
    let name = CString::new(path.as_os_str().as_bytes()).expect("fifo path");
    let result = unsafe { libc::mkfifo(name.as_ptr(), 0o600) };
    assert_eq!(result, 0, "mkfifo failed");
}
