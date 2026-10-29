use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

pub fn await_flag(flag: &AtomicBool, wait: Duration) -> bool {
    let until = Instant::now() + wait;
    while Instant::now() < until {
        if flag.load(Ordering::SeqCst) {
            return true;
        }
        sleep(Duration::from_millis(10));
    }
    flag.load(Ordering::SeqCst)
}
