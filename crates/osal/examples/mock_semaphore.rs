//! Mock semaphore example — counting and binary semaphore usage.
//!
//! Run with:
//! ```bash
//! cargo run --example mock_semaphore --no-default-features --features backend-mock
//! ```

use osal::prelude::*;

fn main() {
    // Counting semaphore: resource pool of 3
    let pool = CountingSemaphore::new(3, 2).unwrap();
    println!("Initial count: {}", pool.count().unwrap());

    pool.acquire(Timeout::NoWait).unwrap();
    pool.acquire(Timeout::NoWait).unwrap();
    println!("After 2 acquires: {}", pool.count().unwrap());

    // Empty — NoWait times out
    assert!(pool.acquire(Timeout::NoWait).is_err());

    // Release one permit
    pool.release().unwrap();
    pool.acquire(Timeout::NoWait).unwrap();
    println!("After release+acquire: {}", pool.count().unwrap());

    // Binary semaphore: task signaling
    let ready = BinarySemaphore::new().unwrap();
    assert!(!ready.is_signaled().unwrap());

    ready.release().unwrap();
    assert!(ready.is_signaled().unwrap());

    ready.acquire(Timeout::NoWait).unwrap();
    assert!(!ready.is_signaled().unwrap());

    println!("Semaphore example complete.");
}
