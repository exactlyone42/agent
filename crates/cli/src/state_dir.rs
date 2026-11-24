#[cfg_attr(unix, path = "state_dir/unix.rs")]
#[cfg_attr(windows, path = "state_dir/windows.rs")]
mod sys;

pub(crate) use sys::{MODE_HINT, create_directory, is_private};
