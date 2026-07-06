//! Mock timer — deterministic software timer with epoch isolation.

use alloc::rc::Rc;
use core::time::Duration;

use osal_api::error::{Error, Result};
use osal_api::traits::timer::{Timer, TimerCallback};
use osal_api::types::TimerMode;

use crate::clock::with_runtime;
use crate::time_runtime::MockTimerKey;

// ---------------------------------------------------------------------------
// Handle inner — Drop deregisters from runtime
// ---------------------------------------------------------------------------

struct MockTimerHandleInner {
    key: MockTimerKey,
}

impl Drop for MockTimerHandleInner {
    fn drop(&mut self) {
        with_runtime(|rt| rt.deregister_timer(self.key));
    }
}

// ---------------------------------------------------------------------------
// MockTimer
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct MockTimer {
    inner: Rc<MockTimerHandleInner>,
}

impl MockTimer {
    pub fn new(_name: &str, period: Duration, mode: TimerMode, callback: TimerCallback) -> Result<Self> {
        if period == Duration::ZERO {
            return Err(Error::InvalidParameter);
        }
        let key = with_runtime(|rt| rt.register_timer(period, mode, callback));
        Ok(Self {
            inner: Rc::new(MockTimerHandleInner { key }),
        })
    }
}

impl Timer for MockTimer {
    fn new(name: &str, period: Duration, mode: TimerMode, callback: TimerCallback) -> Result<Self> {
        Self::new(name, period, mode, callback)
    }

    fn start(&self) -> Result<()> {
        with_runtime(|rt| rt.start_timer(self.inner.key));
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        with_runtime(|rt| rt.stop_timer(self.inner.key));
        Ok(())
    }

    fn reset(&self) -> Result<()> {
        with_runtime(|rt| rt.reset_timer(self.inner.key));
        Ok(())
    }

    fn change_period(&self, new_period: Duration) -> Result<()> {
        if new_period == Duration::ZERO {
            return Err(Error::InvalidParameter);
        }
        with_runtime(|rt| rt.change_period(self.inner.key, new_period));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

pub struct MockTimerFactory;

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
        let actions = with_runtime(|rt| {
            rt.advance_time(d);
            rt.collect_expired_actions()
        });
        for (key, mut cb) in actions {
            cb();
            with_runtime(|rt| rt.restore_callback(key, cb));
        }
    }
}
