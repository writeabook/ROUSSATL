//! Thin wrapper around `pthread_mutex_t`.

use core::cell::UnsafeCell;
use core::time::Duration;

use osal_api::error::Result;

use crate::sys::errno;

// ---------------------------------------------------------------------------
// MutexAttr — RAII wrapper for pthread_mutexattr_t
// ---------------------------------------------------------------------------

struct MutexAttr {
    inner: libc::pthread_mutexattr_t,
    initialized: bool,
}

impl MutexAttr {
    fn new() -> Result<Self> {
        let mut attr = Self {
            inner: unsafe { core::mem::zeroed() },
            initialized: false,
        };
        errno::check_ret(unsafe { libc::pthread_mutexattr_init(&mut attr.inner) })?;
        attr.initialized = true;
        Ok(attr)
    }
}

impl Drop for MutexAttr {
    fn drop(&mut self) {
        if self.initialized {
            unsafe {
                libc::pthread_mutexattr_destroy(&mut self.inner);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// PosixMutex
// ---------------------------------------------------------------------------

/// Wrapper around `pthread_mutex_t`.
///
/// Uses `PTHREAD_MUTEX_ERRORCHECK` for deadlock detection.
/// Non-recursive: a second lock from the same thread returns
/// `EDEADLK`.
///
/// The inner FFI object is wrapped in [`UnsafeCell`] because
/// pthread operations mutate it through `&self`.
pub struct PosixMutex {
    inner: UnsafeCell<libc::pthread_mutex_t>,
}

impl PosixMutex {
    /// Create and initialize a new non-recursive error-checking mutex.
    pub fn new() -> Result<Self> {
        let attr = MutexAttr::new()?;

        errno::check_ret(unsafe {
            libc::pthread_mutexattr_settype(
                &raw const attr.inner as *mut _,
                libc::PTHREAD_MUTEX_ERRORCHECK,
            )
        })?;

        let m = Self {
            inner: UnsafeCell::new(unsafe { core::mem::zeroed() }),
        };

        errno::check_ret(unsafe { libc::pthread_mutex_init(m.raw_ptr(), &attr.inner) })?;

        Ok(m)
    }

    /// Lock the mutex. Blocks until acquired.
    pub fn lock(&self) -> Result<()> {
        errno::check_ret(unsafe { libc::pthread_mutex_lock(self.raw_ptr()) })
    }

    /// Non-blocking try-lock.
    ///
    /// Returns `Ok(())` if acquired, or an error (typically
    /// `Error::LockFailed` via `EBUSY`) if the mutex is held
    /// by another thread.
    pub fn try_lock(&self) -> Result<()> {
        errno::check_ret(unsafe { libc::pthread_mutex_trylock(self.raw_ptr()) })
    }

    /// Timed lock with monotonic deadline.
    ///
    /// Uses `clock_gettime(CLOCK_MONOTONIC)` and `try_lock` with
    /// `nanosleep` backoff to ensure the timeout is measured against
    /// a monotonic clock, regardless of the platform's default
    /// `pthread_mutex_timedlock` clock behavior.
    ///
    /// Returns `Error::Timeout` if the deadline expires before
    /// the lock is acquired.
    pub fn timed_lock(&self, timeout: Duration) -> Result<()> {
        use crate::sys::time;

        let deadline = time::monotonic_now_raw();
        // Compute absolute deadline.
        let deadline = libc::timespec {
            tv_sec: deadline
                .tv_sec
                .saturating_add(timeout.as_secs() as libc::time_t),
            tv_nsec: deadline.tv_nsec + timeout.subsec_nanos() as libc::c_long,
        };
        // Normalize nsec carry-over.
        let deadline = libc::timespec {
            tv_sec: deadline.tv_sec + deadline.tv_nsec / 1_000_000_000,
            tv_nsec: deadline.tv_nsec % 1_000_000_000,
        };

        loop {
            match self.try_lock() {
                Ok(()) => return Ok(()),
                Err(osal_api::error::Error::LockFailed) => {
                    let now = time::monotonic_now_raw();
                    if time::timespec_ge(&now, &deadline) {
                        return Err(osal_api::error::Error::Timeout);
                    }
                    // Small sleep to avoid busy-wait.
                    time::nanosleep(core::time::Duration::from_millis(1));
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Unlock the mutex.
    pub fn unlock(&self) -> Result<()> {
        errno::check_ret(unsafe { libc::pthread_mutex_unlock(self.raw_ptr()) })
    }

    /// Lock and return a RAII guard that unlocks on drop.
    #[allow(dead_code)]
    pub(crate) fn lock_guard(&self) -> Result<PosixMutexGuard<'_>> {
        self.lock()?;
        Ok(PosixMutexGuard {
            mutex: self,
            locked: true,
        })
    }

    /// Return a raw pointer to the inner mutex.
    pub(crate) fn raw_ptr(&self) -> *mut libc::pthread_mutex_t {
        self.inner.get()
    }
}

impl Drop for PosixMutex {
    fn drop(&mut self) {
        unsafe {
            libc::pthread_mutex_destroy(self.raw_ptr());
        }
    }
}

// Safety: pthread_mutex_t is thread-safe.
unsafe impl Send for PosixMutex {}
unsafe impl Sync for PosixMutex {}

// ---------------------------------------------------------------------------
// RAII guard
// ---------------------------------------------------------------------------

/// RAII guard that unlocks the mutex on drop.
pub struct PosixMutexGuard<'a> {
    mutex: &'a PosixMutex,
    locked: bool,
}

impl PosixMutexGuard<'_> {
    /// Return a reference to the underlying mutex.
    #[allow(dead_code)]
    pub(crate) fn mutex(&self) -> &PosixMutex {
        self.mutex
    }

    /// Return a raw mutex pointer for condvar operations.
    pub(crate) fn raw_mutex_ptr(&self) -> *mut libc::pthread_mutex_t {
        self.mutex.raw_ptr()
    }
}

impl Drop for PosixMutexGuard<'_> {
    fn drop(&mut self) {
        if self.locked {
            let _ = self.mutex.unlock();
            self.locked = false;
        }
    }
}
