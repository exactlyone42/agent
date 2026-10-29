use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn canary() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let seed = format!("{nanos}-{}", std::process::id());
    let first = Sha256::digest(seed.as_bytes());
    let second = Sha256::digest(first);
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let body: String = first
        .iter()
        .chain(second.iter())
        .take(36)
        .map(|byte| alphabet[usize::from(*byte) % alphabet.len()] as char)
        .collect();
    format!("ghp_{body}")
}
