//! Mock clock — deterministic virtual time via shared runtime.

use alloc::boxed::Box;
use core::time::Duration;

use osal_api::traits::clock::Clock;

use crate::time_runtime::MockTimeRuntime;

// Raw pointer for single-threaded mock access. Tests are serialized.
static mut RUNTIME_PTR: *mut MockTimeRuntime = core::ptr::null_mut();

pub(crate) fn with_runtime<F, R>(f: F) -> R
where
    F: FnOnce(&mut MockTimeRuntime) -> R,
{
    // Safety: mock is single-threaded; tests are serialized by the harness.
    unsafe { f(&mut *RUNTIME_PTR) }
}

/// Initialize the shared runtime. Call once before any mock tests.
pub fn init_runtime() {
    unsafe {
        if !RUNTIME_PTR.is_null() {
            drop(Box::from_raw(RUNTIME_PTR));
        }
        RUNTIME_PTR = Box::into_raw(Box::new(MockTimeRuntime::new()));
    }
}

/// Reset the runtime between tests. Initializes if not yet done.
pub fn reset_runtime() {
    let ptr = unsafe { RUNTIME_PTR };
    if ptr.is_null() {
        init_runtime();
    } else {
        with_runtime(|rt| rt.reset());
    }
}

// ---------------------------------------------------------------------------
// MockClock
// ---------------------------------------------------------------------------

pub struct MockClock;

impl Clock for MockClock {
    fn now() -> Duration {
        with_runtime(|rt| rt.now())
    }
    fn delay(duration: Duration) {
        with_runtime(|rt| rt.advance(duration));
    }
}

// ---------------------------------------------------------------------------
// MockClockControl
// ---------------------------------------------------------------------------

pub struct MockClockControl;

impl MockClockControl {
    pub fn reset(&self) {
        reset_runtime();
    }
}

#[cfg(feature = "testkit")]
impl osal_testkit::factory::ClockControl for MockClockControl {
    fn advance_clock(&self, d: Duration) {
        with_runtime(|rt| rt.advance(d));
    }
}

#[cfg(feature = "testkit")]
impl osal_testkit::factory::ClockFactory for MockClockControl {
    type Clock = MockClock;
}
