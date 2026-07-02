//! BackendFactory — decouples test logic from concrete backend types.
//!
//! Every backend provides one implementation of this trait. Contract
//! tests are generic over `F: BackendFactory` and call factory methods
//! to obtain the primitives they test.

use core::time::Duration;

use osal_api::error::Result;
use osal_api::traits::mutex::Mutex;
use osal_api::traits::queue::Queue;
use osal_api::traits::semaphore::{BinarySemaphore, CountingSemaphore};

/// Factory for creating OSAL primitives in a backend-agnostic way.
///
/// Each backend (Mock, POSIX, FreeRTOS) provides one implementation.
/// Contract tests are generic over this trait, so the same test code
/// runs against every backend without modification.
///
/// # Associated types
///
/// Each associated type binds to the concrete type from the active
/// backend. This ensures static dispatch — no `Box`, no vtables,
/// compatible with `no_std` environments.
///
/// # Examples
///
/// ```ignore
/// struct MockFactory;
///
/// impl BackendFactory for MockFactory {
///     type Mutex = mock::Mutex<u32>;
///     // ...
/// }
/// ```
pub trait BackendFactory {
    /// Concrete mutex type for `u32` values.
    type Mutex: Mutex<u32>;
    /// Concrete counting semaphore type.
    type CountingSemaphore: CountingSemaphore;
    /// Concrete binary semaphore type.
    type BinarySemaphore: BinarySemaphore;
    /// Concrete queue type.
    type Queue: Queue;

    // ---- creation methods ----

    /// Create a mutex containing `value`.
    fn create_mutex(&self, value: u32) -> Result<Self::Mutex>;

    /// Create a counting semaphore with the given bounds.
    fn create_counting_semaphore(&self, max: u32, initial: u32) -> Result<Self::CountingSemaphore>;

    /// Create a binary semaphore (initial count = 0).
    fn create_binary_semaphore(&self) -> Result<Self::BinarySemaphore>;

    /// Create a queue with the given capacity and message size.
    fn create_queue(&self, capacity: usize, msg_size: usize) -> Result<Self::Queue>;

    // ---- test hooks (optional, default no-op) ----

    /// Hint that the scheduler should yield to other tasks.
    ///
    /// Used in concurrency tests to increase the chance of context
    /// switches between tasks. Backends without cooperative scheduling
    /// support leave this as a no-op.
    fn yield_hint(&self) {}

    /// Advance a fake clock by `duration`.
    ///
    /// Only meaningful for backends with deterministic time (Mock).
    /// Real-time backends leave this as a no-op.
    fn advance_clock(&self, _duration: Duration) {}
}
