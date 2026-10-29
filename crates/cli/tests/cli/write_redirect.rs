use std::io::Write;
use std::net::TcpStream;

pub fn write_redirect(stream: &mut TcpStream) {
    let port = stream
        .local_addr()
        .map(|address| address.port())
        .unwrap_or(0);
    let head = format!(
        "HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{port}/redirected\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.write_all(head.as_bytes());
}
