use std::io::{ErrorKind, Read};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub fn hold_until_disconnect(
    stream: &mut TcpStream,
    ready: &AtomicBool,
    disconnected: &AtomicBool,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    ready.store(true, Ordering::SeqCst);
    let mut byte = [0u8; 1];
    let closed = match stream.read(&mut byte) {
        Ok(0) => true,
        Err(error) => error.kind() == ErrorKind::ConnectionReset,
        Ok(_) => false,
    };
    if closed {
        disconnected.store(true, Ordering::SeqCst);
    }
}
