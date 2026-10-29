use std::os::unix::fs::PermissionsExt;
use std::path::Path;

pub fn set_mode(path: &Path, mode: u32) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("set mode");
}
