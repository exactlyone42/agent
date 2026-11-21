use crate::drift::{file_group::FileGroup, review_state::ReviewState, selection::Selection};

pub const MAX_FILES_PER_RUN: usize = 8;

pub fn select_files(
    groups: Vec<FileGroup>,
    state: &ReviewState,
    now: u64,
    limit: usize,
) -> Selection {
    let total = groups.len();
    let mut due: Vec<FileGroup> = groups
        .into_iter()
        .filter(|group| !state.is_fresh(&group.path, &group.hash, now))
        .collect();
    let fresh = total - due.len();
    due.sort_by(|left, right| {
        state
            .last_reviewed(&left.path)
            .cmp(&state.last_reviewed(&right.path))
            .then_with(|| left.path.cmp(&right.path))
    });
    let deferred = due.len().saturating_sub(limit);
    due.truncate(limit);
    Selection {
        chosen: due,
        fresh,
        deferred,
    }
}
