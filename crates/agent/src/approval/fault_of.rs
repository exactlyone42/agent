use crate::prelude::*;
use agent_harness_approval::Fault;

pub fn fault_of(error: StoreError) -> Fault {
    match error {
        StoreError::InvalidToolCall => Fault::Invalid,
        StoreError::Database(rusqlite::Error::SqliteFailure(code, _))
            if code.code == rusqlite::ErrorCode::DatabaseBusy =>
        {
            Fault::Busy
        }
        _ => Fault::Failed,
    }
}
