//! Thin wrapper around `pthread_create` / `pthread_join`.
//!
//! POSIX task creation and joining, isolated from the higher-level
//! [`crate::task`] completion-state logic.
//!
//! # Portability note
//!
//! This module uses plain `pthread_join`, not `pthread_timedjoin_np`.
//! Timeout join is implemented in [`crate::task`] through a backend
//! completion state + `pthread_cond_timedwait`.

use core::ffi::c_void;
use core::mem::MaybeUninit;

use osal_api::error::{Error, Result};

/// The C ABI entry point for a spawned task.
pub type RawTaskEntry = extern "C" fn(*mut c_void) -> *mut c_void;

/// Configuration for a new POSIX thread.
pub struct PosixThreadConfig {
    /// Requested minimum stack size in bytes.  The backend rounds up
    /// to the platform minimum (`PTHREAD_STACK_MIN`) when lower.
    pub stack_size: usize,
}

/// An OS thread created via `pthread_create`.
pub struct PosixThread {
    tid: libc::pthread_t,
}

impl PosixThread {
    /// Spawn a new thread with the given config, entry, and argument.
    pub fn spawn(
        config: &PosixThreadConfig,
        entry: RawTaskEntry,
        arg: *mut c_void,
    ) -> Result<Self> {
        unsafe {
            let mut attr: libc::pthread_attr_t = MaybeUninit::zeroed().assume_init();
            let mut rc = libc::pthread_attr_init(&raw mut attr);
            debug_assert_eq!(rc, 0, "pthread_attr_init failed");

            // Set stack size, rounding up to platform minimum.
            let stack = if config.stack_size < libc::PTHREAD_STACK_MIN {
                libc::PTHREAD_STACK_MIN
            } else {
                config.stack_size
            };
            rc = libc::pthread_attr_setstacksize(&raw mut attr, stack);
            debug_assert_eq!(rc, 0, "pthread_attr_setstacksize failed");

            let mut tid = MaybeUninit::<libc::pthread_t>::uninit();
            rc = libc::pthread_create(tid.as_mut_ptr(), &attr, entry, arg);
            libc::pthread_attr_destroy(&raw mut attr);

            if rc != 0 {
                return Err(Error::OutOfMemory);
            }

            Ok(Self {
                tid: tid.assume_init(),
            })
        }
    }

    /// Join the thread, consuming this handle.
    ///
    /// Must be called at most once — `pthread_join` is not repeatable.
    /// The higher-level [`crate::task::PosixTask`] guards this with a
    /// completion-state machine.
    pub fn join(self) -> Result<()> {
        let rc = unsafe { libc::pthread_join(self.tid, core::ptr::null_mut()) };

        if rc == 0 {
            Ok(())
        } else {
            Err(Error::Internal("pthread_join failed"))
        }
    }

    /// Detach the thread, consuming this handle.
    ///
    /// After detach, the thread's resources are automatically reclaimed
    /// when it exits — no `join` is needed. Use this when dropping a
    /// `Task` handle without having joined first.
    pub fn detach(self) -> Result<()> {
        let rc = unsafe { libc::pthread_detach(self.tid) };

        if rc == 0 {
            Ok(())
        } else {
            Err(Error::Internal("pthread_detach failed"))
        }
    }
}
