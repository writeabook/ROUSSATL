//! Thin wrapper around `pthread_mutex_t`.

use osal_api::error::Result;

use crate::sys::errno;

/// Wrapper around `pthread_mutex_t`.
///
/// Uses `PTHREAD_MUTEX_ERRORCHECK` for deadlock detection.
pub struct PosixMutex {
    pub(crate) inner: libc::pthread_mutex_t,
}

impl PosixMutex {
    /// Create and initialize a new mutex.
    pub fn new() -> Result<Self> {
        let mut m = Self {
            inner: unsafe { core::mem::zeroed() },
        };
        let mut attr: libc::pthread_mutexattr_t = unsafe { core::mem::zeroed() };
        errno::check_ret(unsafe {
            libc::pthread_mutexattr_init(&mut attr)
        })?;
        errno::check_ret(unsafe {
            libc::pthread_mutexattr_settype(&mut attr, libc::PTHREAD_MUTEX_ERRORCHECK)
        })?;
        errno::check_ret(unsafe {
            libc::pthread_mutex_init(&mut m.inner, &attr)
        })?;
        errno::check_ret(unsafe {
            libc::pthread_mutexattr_destroy(&mut attr)
        })?;
        Ok(m)
    }

    /// Lock the mutex. Blocks until acquired.
    pub fn lock(&self) -> Result<()> {
        errno::check_ret(unsafe { libc::pthread_mutex_lock(&raw const self.inner as *mut _) })
    }

    /// Unlock the mutex.
    pub fn unlock(&self) -> Result<()> {
        errno::check_ret(unsafe { libc::pthread_mutex_unlock(&raw const self.inner as *mut _) })
    }

    /// Lock and return a RAII guard that unlocks on drop.
    pub fn lock_guard(&self) -> Result<PosixMutexGuard<'_>> {
        self.lock()?;
        Ok(PosixMutexGuard {
            mutex: self,
            locked: true,
        })
    }

    /// Return a raw pointer to the inner mutex (for condvar).
    pub(crate) fn raw_ptr(&self) -> *mut libc::pthread_mutex_t {
        &raw const self.inner as *mut _
    }
}

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

// ---------------------------------------------------------------------------
// Drop
// ---------------------------------------------------------------------------

impl Drop for PosixMutex {
    fn drop(&mut self) {
        unsafe {
            libc::pthread_mutex_destroy(&mut self.inner);
        }
    }
}

// Safety: pthread_mutex_t is thread-safe by design.
unsafe impl Send for PosixMutex {}
unsafe impl Sync for PosixMutex {}
