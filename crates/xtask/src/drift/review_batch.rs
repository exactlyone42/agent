use crate::drift::{
    file_group::FileGroup, file_result::FileResult, review_file::review_file,
    review_state::ReviewState, reviewers::Reviewers,
};
use crate::prelude::*;
use std::collections::BTreeMap;

pub struct BatchOutcome {
    pub results: Vec<FileResult>,
    pub failure: Option<String>,
}

pub fn review_batch(
    groups: &[FileGroup],
    files: &BTreeMap<String, Vec<u8>>,
    reviewers: &mut Reviewers,
    state: &mut ReviewState,
    now: u64,
) -> Result<BatchOutcome, XtaskError> {
    let mut results = Vec::new();
    for (index, group) in groups.iter().enumerate() {
        match review_file(group, files, reviewers, index + 1) {
            Ok(result) => results.push(result),
            Err(XtaskError::ReviewTool(message)) => {
                return Ok(BatchOutcome {
                    results,
                    failure: Some(format!("{}: {message}", group.path)),
                });
            }
            Err(error) => return Err(error),
        }
        state.mark(&group.path, &group.hash, now);
    }
    Ok(BatchOutcome {
        results,
        failure: None,
    })
}
