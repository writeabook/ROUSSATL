//! Mock mutex example — basic lock/unlock and clone sharing.
//!
//! Run with:
//! ```bash
//! cargo run --example mock_mutex --no-default-features --features backend-mock
//! ```

use osal::prelude::*;

fn main() {
    let m = Mutex::new(0u32).unwrap();

    // Basic lock/unlock with mutable access
    {
        let mut guard = m.lock(Timeout::NoWait).unwrap();
        *guard += 1;
        println!("Value: {}", *guard);
    }

    // Clone shares the same protected data
    let m2 = m.clone();
    {
        let mut guard = m2.lock(Timeout::NoWait).unwrap();
        *guard = 42;
    }

    // Original handle sees the change
    let guard = m.lock(Timeout::Forever).unwrap();
    println!("Final value: {}", *guard);
}
