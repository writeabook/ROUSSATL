//! POSIX semaphore example — cross-thread signaling.
//!
//! Run with:
//! ```bash
//! cargo run --example posix_semaphore
//! ```

use std::thread;
use std::time::Duration;

use osal::prelude::*;

fn main() {
    // Counting semaphore: cross-thread coordination
    let sem = CountingSemaphore::new(1, 0).unwrap();
    let clone = sem.clone();

    let handle = thread::spawn(move || {
        println!("Worker: waiting for signal...");
        clone.acquire(Timeout::Forever).unwrap();
        println!("Worker: got signal!");
    });

    thread::sleep(Duration::from_millis(10));
    println!("Main: releasing signal");
    sem.release().unwrap();
    handle.join().unwrap();

    println!("Counting semaphore example complete.");

    // Binary semaphore
    let ready = BinarySemaphore::new().unwrap();
    ready.release().unwrap();
    println!("Binary: signaled = {}", ready.is_signaled().unwrap());
    ready.acquire(Timeout::NoWait).unwrap();
    println!("Binary: signaled = {}", ready.is_signaled().unwrap());

    println!("Semaphore example complete.");
}
