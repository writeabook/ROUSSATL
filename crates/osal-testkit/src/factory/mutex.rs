//! Factory for creating [`Mutex`] instances.

use osal_api::error::Result;
use osal_api::traits::mutex::Mutex;

/// Factory for creating mutex instances in a backend-agnostic way.
pub trait MutexFactory {
    /// Concrete mutex type for `u32` values.
    type Mutex: Mutex<u32>;

    /// Create a mutex containing `value`.
    fn create_mutex(&self, value: u32) -> Result<Self::Mutex>;
}
