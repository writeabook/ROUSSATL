//! Time and timeout types for OSAL operations.
//!
//! Uses `core::time::Duration` as the primary time representation
//! (available in `no_std` since Rust 1.32). The `Timeout` enum provides
//! clear semantics for blocking and non-blocking operations.

use core::time::Duration;

/// Timeout strategy for blocking operations.
///
/// Instead of passing raw tick values or magic numbers, callers use
/// this enum to express their intent clearly:
///
/// - `NoWait` — return immediately, never block
/// - `After(d)` — block for at most `d` duration
/// - `Forever` — block indefinitely until the operation succeeds
///
/// # Examples
///
/// ```ignore
/// use core::time::Duration;
/// use osal_api::time::Timeout;
///
/// // Non-blocking receive
/// queue.recv(Timeout::NoWait)?;
///
/// // Block with timeout
/// queue.recv(Timeout::After(Duration::from_millis(100)))?;
///
/// // Block forever
/// queue.recv(Timeout::Forever)?;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timeout {
    /// Do not block — return immediately.
    NoWait,
    /// Block for at most the specified duration.
    After(Duration),
    /// Block indefinitely until the operation succeeds.
    Forever,
}

impl From<Duration> for Timeout {
    fn from(d: Duration) -> Self {
        if d.is_zero() {
            Timeout::NoWait
        } else {
            Timeout::After(d)
        }
    }
}

impl Timeout {
    /// Returns `true` if this is `Timeout::NoWait`.
    pub fn is_no_wait(&self) -> bool {
        matches!(self, Timeout::NoWait)
    }

    /// Returns `true` if this is `Timeout::Forever`.
    pub fn is_forever(&self) -> bool {
        matches!(self, Timeout::Forever)
    }

    /// Returns the duration if this is `Timeout::After`, otherwise `None`.
    pub fn duration(&self) -> Option<Duration> {
        match self {
            Timeout::After(d) => Some(*d),
            _ => None,
        }
    }
}
