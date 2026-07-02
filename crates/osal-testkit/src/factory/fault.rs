//! Fault injection interfaces for error-path testing.
//!
//! All methods default to no-ops, so non-Mock backends can implement
//! this trait without any changes.

/// Factory for injecting faults into OSAL operations.
///
/// Used by Mock backends to simulate error conditions (OOM, timeout,
/// backend failures). Production backends can safely ignore this trait.
pub trait FaultFactory {
    /// Clear all pending fault configurations.
    fn clear_faults(&self);

    /// Cause the next queue creation to fail with the given error.
    fn fail_next_queue_create(&self, _error: osal_api::error::Error) {}

    /// Cause the next queue send to fail with the given error.
    fn fail_next_queue_send(&self, _error: osal_api::error::Error) {}

    /// Cause the next timer creation to fail with the given error.
    fn fail_next_timer_create(&self, _error: osal_api::error::Error) {}
}
