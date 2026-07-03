//! POSIX return code → `osal_api::Error` mapping.

use osal_api::error::{Error, Result};

/// Map POSIX return codes (pthread, etc.) to OSAL errors.
///
/// pthread functions return 0 on success or an error number directly;
/// they do **not** set `errno`. This function maps the return code.
pub fn check_ret(ret: i32) -> Result<()> {
    if ret == 0 {
        Ok(())
    } else {
        Err(map_errno_code(ret))
    }
}

/// Map a POSIX error code to an OSAL error.
pub fn map_errno_code(code: i32) -> Error {
    match code {
        libc::ETIMEDOUT => Error::Timeout,
        libc::EAGAIN => Error::Timeout,
        libc::ENOMEM => Error::OutOfMemory,
        libc::EINVAL => Error::InvalidParameter,
        libc::EBUSY => Error::LockFailed,
        libc::EDEADLK => Error::LockFailed,
        libc::EPERM => Error::LockFailed,
        _ => Error::Internal("unexpected POSIX error"),
    }
}

/// Map the current `errno` to an OSAL error.
///
/// Use only with POSIX functions that set `errno` on failure
/// (e.g. `clock_gettime`, `sem_wait`). Do **not** use with
/// pthread mutex / condvar functions.
#[allow(dead_code)]
pub fn check_errno_ret(ret: i32) -> Result<()> {
    if ret == 0 {
        Ok(())
    } else {
        Err(map_current_errno())
    }
}

fn map_current_errno() -> Error {
    let e = unsafe { *libc::__errno_location() };
    map_errno_code(e)
}
