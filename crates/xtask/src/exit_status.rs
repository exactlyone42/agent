use crate::error::XtaskError;

pub fn exit_status(result: Result<i32, XtaskError>) -> u8 {
    match result {
        Ok(0) => 0,
        Ok(_) => 1,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}
