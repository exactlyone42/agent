use crate::fake_server::Handler;
use crate::serve_connection::serve_connection;
use serde_json::Value;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;

pub fn accept_loop(
    listener: TcpListener,
    handler: Handler,
    requests: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
) {
    if listener.set_nonblocking(true).is_err() {
        return;
    }
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                let handler = handler.clone();
                let requests = requests.clone();
                spawn(move || serve_connection(stream, handler, requests));
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => sleep(Duration::from_millis(5)),
            Err(_) => return,
        }
    }
}
