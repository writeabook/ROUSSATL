//! Mock clock — deterministic virtual time via thread-local runtime.

use core::cell::RefCell;
use core::time::Duration;

use osal_api::traits::clock::Clock;

use crate::time_runtime::MockTimeRuntime;

thread_local! {
    static MOCK_RUNTIME: RefCell<Option<MockTimeRuntime>> = RefCell::new(None);
}

fn with_runtime<F, R>(f: F) -> R
where
    F: FnOnce(&mut MockTimeRuntime) -> R,
{
    MOCK_RUNTIME.with(|cell| {
        let mut rt = cell.borrow_mut();
        let rt = rt.get_or_insert_with(MockTimeRuntime::new);
        f(rt)
    })
}

/// Reset the runtime between tests.
pub fn reset_runtime() {
    MOCK_RUNTIME.with(|cell| {
        if let Some(rt) = cell.borrow_mut().as_mut() {
            rt.reset();
        }
    });
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
        // Must execute callbacks outside the RefCell borrow.
        // advance returns the runtime, we execute callbacks after.
        let actions = with_runtime(|rt| {
            rt.advance_time(duration);
            rt.collect_expired_actions()
        });
        for (id, mut cb) in actions {
            cb();
            with_runtime(|rt| rt.restore_callback(id, cb));
        }
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
        // Same as delay: collect actions inside borrow, execute outside
        let actions = with_runtime(|rt| {
            rt.advance_time(d);
            rt.collect_expired_actions()
        });
        for (id, mut cb) in actions {
            cb();
            with_runtime(|rt| rt.restore_callback(id, cb));
        }
    }
}

#[cfg(feature = "testkit")]
impl osal_testkit::factory::ClockFactory for MockClockControl {
    type Clock = MockClock;
}
