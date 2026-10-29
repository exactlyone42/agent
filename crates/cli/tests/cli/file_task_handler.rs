use crate::completion::completion;
use crate::planned_call::planned_call;
use crate::text_message::text_message;
use crate::tool_call_message::tool_call_message;
use crate::write_json::write_json;
use crate::write_redirect::write_redirect;
use serde_json::Value;
use std::net::TcpStream;
use std::path::PathBuf;

pub fn file_task_handler(
    name: &'static str,
    source: PathBuf,
    database: PathBuf,
    output: String,
) -> impl Fn(usize, &Value, &mut TcpStream) + Send + Sync + 'static {
    move |index, _, stream| {
        if name == "redirect" {
            write_redirect(stream);
            return;
        }
        let reply = match planned_call(name, index, &source, &database, &output) {
            Some((tool, arguments)) => {
                let id = if name == "reused_call_id" {
                    "reused-id".to_string()
                } else {
                    format!("call_{index}")
                };
                completion(tool_call_message(&id, tool, &arguments), "tool_calls")
            }
            None => completion(text_message("done"), "stop"),
        };
        write_json(stream, &reply);
    }
}
