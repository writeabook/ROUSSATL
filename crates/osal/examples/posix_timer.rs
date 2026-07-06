//! POSIX timer example — periodic timer with stop.
//!
//! Run with:
//! ```bash
//! cargo run --example posix_timer
//! ```

use core::time::Duration;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use osal::prelude::*;

fn main() {
    let count = Arc::new(AtomicU32::new(0));
    let c = count.clone();

    let t = Timer::new(
        "example",
        Duration::from_millis(100),
        TimerMode::Periodic,
        Box::new(move || { c.fetch_add(1, Ordering::Relaxed); }),
    )
    .unwrap();

    t.start().unwrap();
    Clock::delay(Duration::from_millis(350));
    t.stop().unwrap();

    let n = count.load(Ordering::Relaxed);
    println!("Timer fired {} times in 350ms", n);

    // No more callbacks after stop
    let after_stop = count.load(Ordering::Relaxed);
    Clock::delay(Duration::from_millis(200));
    assert_eq!(count.load(Ordering::Relaxed), after_stop);

    println!("POSIX timer example complete.");
}
