//! Prelude module for convenient importing.
//!
//! Exports the most commonly used and stable types so users can write:
//!
//! ```ignore
//! use osal::prelude::*;
//! ```
//!
//! Only the public API surface is exported here. Backend internals,
//! shared implementation details, and experimental types are
//! intentionally excluded.

pub use crate::error::{Error, Result};
pub use crate::time::Timeout;

// Phase 2 additions:
// pub use crate::traits::mutex::Mutex;
// pub use crate::traits::semaphore::{BinarySemaphore, CountingSemaphore};
// pub use crate::traits::queue::Queue;
// pub use crate::traits::task::{Task, TaskBuilder};
// pub use crate::traits::timer::Timer;
// pub use crate::traits::event_flags::EventFlags;
// pub use crate::traits::system::System;
