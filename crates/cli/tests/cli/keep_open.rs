use std::os::fd::RawFd;
use std::os::unix::process::CommandExt;
use std::process::Command;

pub fn keep_open(command: &mut Command, descriptors: Vec<RawFd>) {
    unsafe {
        command.pre_exec(move || {
            for descriptor in &descriptors {
                libc::fcntl(*descriptor, libc::F_SETFD, 0);
            }
            Ok(())
        });
    }
}
