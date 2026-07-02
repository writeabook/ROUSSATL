//! Factory for creating [`Queue`] instances.

use osal_api::error::Result;
use osal_api::traits::queue::Queue;

/// Factory for creating queue instances in a backend-agnostic way.
pub trait QueueFactory {
    /// Concrete queue type.
    type Queue: Queue;

    /// Create a queue with the given capacity and message size.
    fn create_queue(&self, capacity: usize, msg_size: usize) -> Result<Self::Queue>;
}
