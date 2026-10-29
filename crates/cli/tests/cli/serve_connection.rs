use crate::fake_server::Handler;
use crate::read_request::read_request;
use serde_json::Value;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

pub fn serve_connection(mut stream: TcpStream, handler: Handler, requests: Arc<Mutex<Vec<Value>>>) {
    if stream.set_nonblocking(false).is_err() {
        return;
    }
    let Some(request) = read_request(&stream) else {
        return;
    };
    let index = match requests.lock() {
        Ok(mut list) => {
            list.push(request.clone());
            list.len()
        }
        Err(_) => return,
    };
    handler(index, &request, &mut stream);
}
