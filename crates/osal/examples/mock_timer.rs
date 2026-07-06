//! Mock timer example — deterministic OneShot and Periodic timers.
//!
//! Run with:
//! ```bash
//! cargo run --example mock_timer --no-default-features --features backend-mock
//! ```

use core::time::Duration;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use osal::prelude::*;

fn main() {
    // Initialize mock runtime
    // (automatically initialized on first use)

    // OneShot timer
    let fired = Arc::new(AtomicU32::new(0));
    let f = fired.clone();
    let t = Timer::new(
        "oneshot",
        Duration::from_millis(100),
        TimerMode::OneShot,
        Box::new(move || {
            f.fetch_add(1, Ordering::Relaxed);
        }),
    )
    .unwrap();

    t.start().unwrap();
    Clock::delay(Duration::from_millis(99));
    assert_eq!(fired.load(Ordering::Relaxed), 0);
    Clock::delay(Duration::from_millis(1));
    assert_eq!(fired.load(Ordering::Relaxed), 1);

    // Periodic timer
    let count = Arc::new(AtomicU32::new(0));
    let c = count.clone();
    let pt = Timer::new(
        "periodic",
        Duration::from_millis(100),
        TimerMode::Periodic,
        Box::new(move || {
            c.fetch_add(1, Ordering::Relaxed);
        }),
    )
    .unwrap();

    pt.start().unwrap();
    Clock::delay(Duration::from_millis(350));
    let n = count.load(Ordering::Relaxed);
    println!("Periodic fired {} times", n);

    pt.stop().unwrap();
    Clock::delay(Duration::from_millis(200));
    assert_eq!(count.load(Ordering::Relaxed), n); // no more fires

    println!("Mock timer example complete.");
}
