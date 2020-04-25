use std::{thread, time::Instant};

pub fn sleep_until(then: Instant) {
    let now = Instant::now();
    if then > now {
        thread::sleep(then - now);
    }
}
