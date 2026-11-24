use crate::prelude::*;

pub fn en_store_text(error: &StoreError) -> &'static str {
    match error {
        StoreError::Database(_) => "Could not save or read the task record",
        StoreError::Incompatible => "The task database version or settings are incompatible",
        StoreError::NotFound => "The task was not found",
        StoreError::Conflict => {
            "The task record has changed; do not save a result from a stale state or run it again"
        }
        StoreError::InvalidCursor => "The event position must not be negative",
        StoreError::InvalidSpec => {
            "The task specification is outside the limits, so it cannot continue"
        }
        StoreError::InvalidToolCall => "The tool name or arguments are outside the limits",
    }
}
