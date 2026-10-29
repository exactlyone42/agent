use std::path::{Path, PathBuf};
use std::process::Command;

pub fn compile_fixture(directory: &Path) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/open_race.c");
    let library = directory.join("fixture.so");
    let result = Command::new("cc")
        .args(["-shared", "-fPIC", "-Wall", "-Wextra", "-Werror"])
        .arg(source)
        .arg("-o")
        .arg(&library)
        .output()
        .expect("cc is required for the injection fixture");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    library
}
