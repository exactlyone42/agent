#[cfg_attr(unix, path = "single_link/unix.rs")]
#[cfg_attr(windows, path = "single_link/windows.rs")]
mod sys;

pub use sys::single_link;
