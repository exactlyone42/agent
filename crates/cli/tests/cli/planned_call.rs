use crate::open_database::open_database;
use serde_json::{Value, json};
use std::io::Write;
use std::path::Path;

fn first_request_effect(name: &str, source: &Path, database: &Path) {
    if name == "corrupted_spec" {
        open_database(database)
            .execute_batch("UPDATE task_specs SET spec_hash='corrupted'")
            .expect("corrupt spec");
    }
    if name == "source_grew" {
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(source)
            .expect("open source");
        file.write_all(b"y").expect("grow source");
    }
}

fn final_request_effect(name: &str, database: &Path) {
    if name == "receipt_failed" {
        open_database(database)
            .execute_batch("CREATE TRIGGER reject_receipt BEFORE INSERT ON events WHEN NEW.event_type='succeeded' BEGIN SELECT RAISE(ABORT, 'injected receipt failure'); END")
            .expect("receipt trigger");
    }
}

pub fn planned_call(
    name: &str,
    index: usize,
    source: &Path,
    database: &Path,
    output: &str,
) -> Option<(&'static str, Value)> {
    match index {
        1 => {
            first_request_effect(name, source, database);
            Some(("read_input", json!({})))
        }
        2 if name != "missing" => {
            let content = if name == "wrong" {
                "incorrect content"
            } else {
                output
            };
            let mut arguments = json!({"content": content});
            if name == "malformed" {
                arguments["path"] = json!("unapproved.txt");
            }
            Some(("write_output", arguments))
        }
        3 if name == "duplicate_write" => Some((
            "write_output",
            json!({"content": "unexpected second write"}),
        )),
        _ => {
            final_request_effect(name, database);
            None
        }
    }
}
