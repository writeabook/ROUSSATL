//! POSIX blocking semaphore — cross-thread signaling example.
//!
//! Demonstrates `CountingSemaphore` acquire/release across threads
//! using the POSIX backend directly.
//!
//! ```bash
//! cargo run -p osal-backend-posix --example blocking_semaphore --features testkit
//! ```

use std::thread;
use std::time::Duration;

use osal_api::time::Timeout;
use osal_api::traits::semaphore::CountingSemaphore as _;
use osal_backend_posix::semaphore::PosixCountingSemaphore;

fn main() {
    let sem = PosixCountingSemaphore::new(1, 0).unwrap();
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

    println!("Blocking semaphore example complete.");
}
