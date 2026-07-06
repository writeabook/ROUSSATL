//! Mock mutex implementation.
//!
//! Uses `Rc` for shared ownership and `UnsafeCell` + `Cell` for
//! interior mutability. Supports recursive locking.
//!
//! # Capability boundary
//!
//! - Core contracts: supported (creation, lock/unlock, recursive)
//! - Blocking contracts: deferred (single execution context;
//!   cross-task contention not simulated)
//!
//! # Timeout semantics
//!
//! - `Timeout::NoWait`: succeeds if uncontended or recursive.
//! - `Timeout::After(_)`: same as NoWait in single-task model.
//! - `Timeout::Forever`: always succeeds (recursive).

use alloc::rc::Rc;
use core::cell::{Cell, UnsafeCell};
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use osal_api::error::Result;
use osal_api::time::Timeout;
use osal_api::traits::mutex::Mutex;

// ---------------------------------------------------------------------------
// Inner state
// ---------------------------------------------------------------------------

struct MockMutexInner<T> {
    /// The protected data.
    data: UnsafeCell<T>,
    /// Recursion depth. 0 = unlocked, 1 = locked once, N = N locks.
    recursion: Cell<usize>,
}

// Safety: the outer Rc ensures single ownership of the allocation.
// UnsafeCell access is guarded by the recursion counter — data is
// only accessed when recursion > 0 (lock held).
unsafe impl<T: Send> Send for MockMutexInner<T> {}
unsafe impl<T: Send> Sync for MockMutexInner<T> {}

// ---------------------------------------------------------------------------
// Public type
// ---------------------------------------------------------------------------

/// A mock mutex for contract testing.
///
/// Uses `Rc` internally; cloned handles share the same backend resource.
pub struct MockMutex<T> {
    inner: Rc<MockMutexInner<T>>,
}

impl<T> Clone for MockMutex<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl<T> MockMutex<T> {
    /// Create a new mock mutex containing `value`.
    pub fn new(value: T) -> Result<Self> {
        Ok(Self {
            inner: Rc::new(MockMutexInner {
                data: UnsafeCell::new(value),
                recursion: Cell::new(0),
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// Guard
// ---------------------------------------------------------------------------

/// RAII guard for [`MockMutex`].
///
/// Provides `&T` / `&mut T` access via [`Deref`] / [`DerefMut`].
/// Decrements the recursion count on drop; releases the lock
/// when the count reaches zero.
///
/// `!Send`: the guard represents ownership of a lock held by the
/// current task. It must not be sent to another thread.
pub struct MockMutexGuard<'a, T> {
    inner: &'a MockMutexInner<T>,
    // Make the guard `!Send`.
    _not_send: PhantomData<*const ()>,
}

impl<T> Deref for MockMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: the guard only exists when recursion > 0,
        // meaning the lock is held. Data access is safe.
        unsafe { &*self.inner.data.get() }
    }
}

impl<T> DerefMut for MockMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: the guard provides exclusive &mut access.
        // In the mock single-context model, no other task can
        // concurrently access the data.
        unsafe { &mut *self.inner.data.get() }
    }
}

impl<T> Drop for MockMutexGuard<'_, T> {
    fn drop(&mut self) {
        let n = self.inner.recursion.get();
        debug_assert!(n > 0, "guard dropped on unlocked mutex");
        self.inner.recursion.set(n - 1);
    }
}

// ---------------------------------------------------------------------------
// Mutex trait
// ---------------------------------------------------------------------------

impl<T: 'static> Mutex<T> for MockMutex<T> {
    type Guard<'a>
        = MockMutexGuard<'a, T>
    where
        Self: 'a,
        T: 'a;

    fn new(value: T) -> Result<Self> {
        Self::new(value)
    }

    fn lock(&self, timeout: Timeout) -> Result<Self::Guard<'_>> {
        let n = self.inner.recursion.get();

        if n > 0 {
            // Already locked by this context — recursive lock.
            self.inner.recursion.set(n + 1);
            return Ok(MockMutexGuard {
    inner: &self.inner,
    _not_send: PhantomData,
});
        }

        match timeout {
            Timeout::NoWait => {
                // Uncontended — acquire.
                self.inner.recursion.set(1);
                Ok(MockMutexGuard {
    inner: &self.inner,
    _not_send: PhantomData,
})
            }
            Timeout::After(d) => {
                if d == core::time::Duration::ZERO {
                    // After(ZERO) on unlocked mutex succeeds immediately.
                    // After(ZERO) on locked mutex would return Timeout,
                    // but in single-task model the lock is always ours.
                    self.inner.recursion.set(1);
                    Ok(MockMutexGuard {
    inner: &self.inner,
    _not_send: PhantomData,
})
                } else {
                    // Non-zero After — succeed immediately in mock.
                    self.inner.recursion.set(1);
                    Ok(MockMutexGuard {
    inner: &self.inner,
    _not_send: PhantomData,
})
                }
            }
            Timeout::Forever => {
                self.inner.recursion.set(1);
                Ok(MockMutexGuard {
    inner: &self.inner,
    _not_send: PhantomData,
})
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Factory (testkit)
// ---------------------------------------------------------------------------

/// Factory for creating mock mutexes.
pub struct MockMutexFactory;

#[cfg(feature = "testkit")]
impl osal_testkit::factory::MutexFactory for MockMutexFactory {
    type Mutex = MockMutex<u32>;

    fn create_mutex(&self, value: u32) -> Result<Self::Mutex> {
        MockMutex::new(value)
    }
}
