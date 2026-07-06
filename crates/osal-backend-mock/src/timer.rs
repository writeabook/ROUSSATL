//! Mock timer — deterministic software timer.
//!
//! Uses `Rc` for handle sharing and delegates all operations to the
//! shared `MockTimeRuntime`.

use alloc::rc::Rc;
use core::time::Duration;

use osal_api::error::Result;
use osal_api::traits::timer::{Timer, TimerCallback};
use osal_api::types::TimerMode;

use crate::clock::with_runtime;

// ---------------------------------------------------------------------------
// MockTimer
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MockTimer {
    id: u64,
    // Rc so cloned handles share the drop-counting
    _handle: Rc<()>,
}

impl MockTimer {
    pub fn new(_name: &str, period: Duration, mode: TimerMode, callback: TimerCallback) -> Result<Self> {
        if period == Duration::ZERO {
            return Err(osal_api::error::Error::InvalidParameter);
        }
        let id = with_runtime(|rt| rt.register_timer(period, mode, callback));
        Ok(Self {
            id,
            _handle: Rc::new(()),
        })
    }
}

impl Timer for MockTimer {
    fn new(name: &str, period: Duration, mode: TimerMode, callback: TimerCallback) -> Result<Self> {
        Self::new(name, period, mode, callback)
    }

    fn start(&self) -> Result<()> {
        with_runtime(|rt| rt.start_timer(self.id));
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        with_runtime(|rt| rt.stop_timer(self.id));
        Ok(())
    }

    fn reset(&self) -> Result<()> {
        with_runtime(|rt| rt.reset_timer(self.id));
        Ok(())
    }

    fn change_period(&self, new_period: Duration) -> Result<()> {
        if new_period == Duration::ZERO {
            return Err(osal_api::error::Error::InvalidParameter);
        }
        with_runtime(|rt| rt.change_period(self.id, new_period));
        Ok(())
    }
}

impl Drop for MockTimer {
    fn drop(&mut self) {
        // Only deregister when the last Rc handle drops
        if Rc::strong_count(&self._handle) == 1 {
            with_runtime(|rt| rt.deregister_timer(self.id));
        }
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

pub struct MockTimerFactory;

#[cfg(feature = "testkit")]
use osal_testkit::factory::{ClockControl, ClockFactory};

#[cfg(feature = "testkit")]
impl osal_testkit::factory::TimerFactory for MockTimerFactory {
    type Timer = MockTimer;

    fn create_timer(
        &self,
        name: &str,
        period: Duration,
        mode: TimerMode,
        callback: TimerCallback,
    ) -> Result<Self::Timer> {
        MockTimer::new(name, period, mode, callback)
    }
}

#[cfg(feature = "testkit")]
impl osal_testkit::factory::ClockFactory for MockTimerFactory {
    type Clock = crate::clock::MockClock;
}

#[cfg(feature = "testkit")]
impl osal_testkit::factory::ClockControl for MockTimerFactory {
    fn advance_clock(&self, d: Duration) {
        crate::clock::with_runtime(|rt| rt.advance(d));
    }
}
