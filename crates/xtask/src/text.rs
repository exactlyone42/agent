use std::path::Path;

pub fn text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
