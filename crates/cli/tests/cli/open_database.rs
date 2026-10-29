use rusqlite::Connection;
use std::path::Path;

pub fn open_database(path: &Path) -> Connection {
    Connection::open(path).expect("open task database")
}
