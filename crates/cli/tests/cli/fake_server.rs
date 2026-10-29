use crate::accept_loop::accept_loop;
use serde_json::Value;
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{JoinHandle, spawn};

pub type Handler = Arc<dyn Fn(usize, &Value, &mut TcpStream) + Send + Sync>;

pub struct FakeServer {
    port: u16,
    requests: Arc<Mutex<Vec<Value>>>,
    stop: Arc<AtomicBool>,
    accept: Option<JoinHandle<()>>,
}

impl FakeServer {
    pub fn start(handler: impl Fn(usize, &Value, &mut TcpStream) + Send + Sync + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake model server");
        let port = listener.local_addr().expect("server address").port();
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let accept = spawn({
            let requests = requests.clone();
            let stop = stop.clone();
            move || accept_loop(listener, Arc::new(handler), requests, stop)
        });
        Self {
            port,
            requests,
            stop,
            accept: Some(accept),
        }
    }

    pub fn endpoint(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.port)
    }

    pub fn requests(&self) -> Vec<Value> {
        self.requests
            .lock()
            .map(|list| list.clone())
            .unwrap_or_default()
    }
}

impl Drop for FakeServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(accept) = self.accept.take() {
            let _ = accept.join();
        }
    }
}
