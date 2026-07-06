//! POSIX mutex example — basic lock/unlock, clone sharing, and timeout.
//!
//! Run with:
//! ```bash
//! cargo run --example posix_mutex
//! ```

use core::time::Duration;

use osal::prelude::*;

fn main() {
    let m = Mutex::new(0u32).unwrap();

    // Basic lock/unlock with mutable access
    {
        let mut guard = m.lock(Timeout::NoWait).unwrap();
        *guard = 42;
        println!("Set value to: {}", *guard);
    }

    // Clone shares the same protected data
    let m2 = m.clone();
    {
        let guard = m2.lock(Timeout::Forever).unwrap();
        println!("Clone sees: {}", *guard);
    }

    // After timeout on uncontended mutex
    let guard = m.lock(Timeout::After(Duration::from_millis(1))).unwrap();
    println!("After succeeded: {}", *guard);
    drop(guard);

    println!("Mutex example complete.");
}
