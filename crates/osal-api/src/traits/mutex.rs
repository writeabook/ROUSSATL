//! Mutex trait — recursive mutual exclusion lock.
//!
//! See [the backend contract](../../../docs/backend-contract.md#8-mutex-contract)
//! for the full behavioral specification.

use core::ops::{Deref, DerefMut};

use crate::error::Result;
use crate::time::Timeout;

/// A recursive mutual exclusion lock protecting a value of type `T`.
///
/// # Recursive semantics
///
/// The owning task may lock the same mutex multiple times without
/// deadlocking. Each call to [`lock`](Mutex::lock) must be paired with
/// exactly one drop of the returned guard. The mutex is fully released
/// only when the last guard is dropped.
///
/// # ISR semantics
///
/// [`isr_lock`](Mutex::isr_lock) is a non-blocking variant. On backends
/// without true ISR context it behaves identically to
/// `lock(Timeout::NoWait)`. On backends where ISR mutex operations are
/// unsupported it returns `Error::Unsupported`.
///
/// # Examples
///
/// ```ignore
/// use osal::prelude::*;
///
/// let counter = PosixMutex::new(0u32)?;
/// {
///     let mut guard = counter.lock(Timeout::Forever)?;
///     *guard += 1;
/// } // lock released here
/// ```
pub trait Mutex<T>: Sized {
    /// The guard type returned by a successful lock.
    ///
    /// Provides `&mut T` access via [`DerefMut`]. Releases one level of
    /// the lock when dropped.
    type Guard<'a>: Deref<Target = T> + DerefMut<Target = T>
    where
        Self: 'a,
        T: 'a;

    /// Create a new mutex containing `value`.
    ///
    /// Returns `Error::OutOfMemory` if the underlying OS resource
    /// cannot be allocated.
    fn new(value: T) -> Result<Self>;

    /// Acquire the lock, blocking according to `timeout`.
    ///
    /// | `timeout` | Behavior |
    /// |-----------|----------|
    /// | `NoWait`  | Return immediately; `Error::LockFailed` if the mutex is held by another task. |
    /// | `After(d)`| Block for at most `d`; `Error::Timeout` if the mutex is not acquired in time. |
    /// | `Forever` | Block until the mutex is acquired. |
    ///
    /// The owning task may call `lock` again without blocking
    /// (recursive lock). Each `lock` call produces a new guard;
    /// dropping that guard releases one recursion level.
    fn lock(&self, timeout: Timeout) -> Result<Self::Guard<'_>>;

    /// Non-blocking lock attempt, safe to call from ISR context.
    ///
    /// Returns `Error::LockFailed` if the mutex is held by another
    /// context. Returns `Error::Unsupported` if the backend does not
    /// support ISR mutex operations.
    fn isr_lock(&self) -> Result<Self::Guard<'_>>;
}
