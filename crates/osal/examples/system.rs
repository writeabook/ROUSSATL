//! Minimal example demonstrating [`System`] operations.
//!
//! Run with:
//! ```bash
//! cargo run -p osal --example system
//! cargo run -p osal --example system --no-default-features --features backend-mock
//! ```

use osal::prelude::*;

fn main() {
    let free = System::heap_free();
    println!("Heap free: {free}");

    // Enter a nested critical section.
    {
        let _outer = System::enter_critical();
        let _inner = System::enter_critical();
        println!("Inside nested critical section");
        // Guards drop in reverse order here, each exiting one level.
    }
    println!("Outside critical section");
}
