use serde_json::Value;
use std::io::Write;
use std::net::TcpStream;

pub fn write_json(stream: &mut TcpStream, body: &Value) {
    let payload = body.to_string();
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        payload.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(payload.as_bytes());
}
