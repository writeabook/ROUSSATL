//! Queue example — basic send / recv roundtrip.
//!
//! Works with any OSAL backend:
//! ```bash
//! cargo run -p osal --example queue
//! cargo run -p osal --example queue --no-default-features --features backend-mock
//! ```

use osal::prelude::*;

fn main() -> Result<()> {
    let q = Queue::new(4, 4)?;

    q.send(&1u32.to_le_bytes(), Timeout::NoWait)?;

    let mut buf = [0u8; 4];
    q.recv(&mut buf, Timeout::NoWait)?;

    let value = u32::from_le_bytes(buf);
    assert_eq!(value, 1);
    println!("Queue roundtrip OK: {value}");
    Ok(())
}
