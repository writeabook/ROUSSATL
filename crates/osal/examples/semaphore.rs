//! Semaphore example — counting and binary semaphore usage.
//!
//! Works with any OSAL backend:
//! ```bash
//! cargo run -p osal --example semaphore
//! cargo run -p osal --example semaphore --no-default-features --features backend-mock
//! ```

use osal::prelude::*;

fn main() -> Result<()> {
    // Counting semaphore: resource pool of 2
    let pool = CountingSemaphore::new(2, 1)?;
    println!("Initial count: {}", pool.count()?);

    pool.acquire(Timeout::NoWait)?;
    println!("After acquire: {}", pool.count()?);

    // Release returns the permit.
    pool.release()?;
    println!("After release: {}", pool.count()?);

    // Binary semaphore: task-to-task signaling
    let ready = BinarySemaphore::new()?;
    assert!(!ready.is_signaled()?);

    ready.release()?;
    assert!(ready.is_signaled()?);

    ready.acquire(Timeout::NoWait)?;
    assert!(!ready.is_signaled()?);

    println!("Semaphore example complete.");
    Ok(())
}
