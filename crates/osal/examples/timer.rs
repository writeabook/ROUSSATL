//! Timer example — OneShot timer with Clock::delay.
//!
//! Works with any OSAL backend (Mock advances virtual time;
//! POSIX uses real monotonic sleep):
//! ```bash
//! cargo run -p osal --example timer
//! cargo run -p osal --example timer --no-default-features --features backend-mock
//! ```

use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;
use std::sync::Arc;

use osal::prelude::*;

fn main() -> Result<()> {
    let fired = Arc::new(AtomicU32::new(0));
    let f = Arc::clone(&fired);

    let timer = Timer::new(
        "example",
        Duration::from_millis(10),
        TimerMode::OneShot,
        Box::new(move || {
            f.fetch_add(1, Ordering::SeqCst);
        }),
    )?;

    timer.start()?;
    Clock::delay(Duration::from_millis(20));

    assert_eq!(fired.load(Ordering::SeqCst), 1);
    println!("Timer fired exactly once.");

    timer.stop()?;
    Ok(())
}
