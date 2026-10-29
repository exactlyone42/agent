use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;
use std::time::Duration;

pub fn read_request(stream: &TcpStream) -> Option<Value> {
    stream.set_read_timeout(Some(Duration::from_secs(5))).ok()?;
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).ok()?;
        let header = line.trim_end().to_ascii_lowercase();
        if header.is_empty() {
            break;
        }
        if let Some(value) = header.strip_prefix("content-length:") {
            length = value.trim().parse().ok()?;
        }
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}
