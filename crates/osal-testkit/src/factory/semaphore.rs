//! Factory for creating semaphore instances.

use osal_api::error::Result;
use osal_api::traits::semaphore::{BinarySemaphore, CountingSemaphore};

/// Factory for creating semaphore instances in a backend-agnostic way.
pub trait SemaphoreFactory {
    /// Concrete counting semaphore type.
    type CountingSemaphore: CountingSemaphore;
    /// Concrete binary semaphore type.
    type BinarySemaphore: BinarySemaphore;

    /// Create a counting semaphore with the given bounds.
    fn create_counting_semaphore(&self, max: u32, initial: u32) -> Result<Self::CountingSemaphore>;

    /// Create a binary semaphore (initial count = 0).
    fn create_binary_semaphore(&self) -> Result<Self::BinarySemaphore>;
}
