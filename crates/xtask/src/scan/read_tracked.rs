use crate::prelude::*;

pub fn read_tracked(
    root: &Path,
    paths: &[String],
) -> Result<BTreeMap<String, Vec<u8>>, XtaskError> {
    let mut files = BTreeMap::new();
    for path in paths {
        match fs::read(root.join(path)) {
            Ok(bytes) => {
                files.insert(path.clone(), bytes);
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(files)
}
