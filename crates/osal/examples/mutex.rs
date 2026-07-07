//! Mutex example — lock / guard / clone sharing.
//!
//! Works with any OSAL backend:
//! ```bash
//! cargo run -p osal --example mutex
//! cargo run -p osal --example mutex --no-default-features --features backend-mock
//! ```

use osal::prelude::*;

fn main() -> Result<()> {
    let counter = Mutex::new(0u32)?;

    // Lock, mutate, release via guard drop.
    {
        let mut guard = counter.lock(Timeout::Forever)?;
        *guard += 1;
    }

    // Clone shares the same protected data.
    let c2 = counter.clone();
    {
        let guard = c2.lock(Timeout::Forever)?;
        println!("Counter: {}", *guard);
        assert_eq!(*guard, 1);
    }

    println!("Mutex example complete.");
    Ok(())
}
