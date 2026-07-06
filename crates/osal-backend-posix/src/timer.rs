//! POSIX timer — software timer backed by `PosixTimerService`.

use alloc::sync::Arc;
use core::time::Duration;

use osal_api::error::{Error, Result};
use osal_api::traits::timer::{Timer, TimerCallback};
use osal_api::types::TimerMode;

use crate::timer_service;

// ---------------------------------------------------------------------------
// PosixTimer
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct PosixTimer {
    id: u64,
    _handle: Arc<()>,
}

impl PosixTimer {
    pub fn new(_name: &str, period: Duration, mode: TimerMode, callback: TimerCallback) -> Result<Self> {
        if period == Duration::ZERO {
            return Err(Error::InvalidParameter);
        }
        let id = timer_service::register(period, mode, callback)
            .ok_or(Error::OutOfMemory)?;
        Ok(Self {
            id,
            _handle: Arc::new(()),
        })
    }
}

impl Timer for PosixTimer {
    fn new(name: &str, period: Duration, mode: TimerMode, callback: TimerCallback) -> Result<Self> {
        Self::new(name, period, mode, callback)
    }

    fn start(&self) -> Result<()> {
        timer_service::start(self.id);
        Ok(())
    }

    fn stop(&self) -> Result<()> {
        timer_service::stop(self.id);
        Ok(())
    }

    fn reset(&self) -> Result<()> {
        timer_service::reset(self.id);
        Ok(())
    }

    fn change_period(&self, new_period: Duration) -> Result<()> {
        if new_period == Duration::ZERO {
            return Err(Error::InvalidParameter);
        }
        timer_service::change_period(self.id, new_period);
        Ok(())
    }
}

impl Drop for PosixTimer {
    fn drop(&mut self) {
        // Only deregister when the last Arc handle drops
        if Arc::strong_count(&self._handle) == 1 {
            timer_service::deregister(self.id);
        }
    }
}

// ---------------------------------------------------------------------------
// Factory
// ---------------------------------------------------------------------------

pub struct PosixTimerFactory;

#[cfg(feature = "testkit")]
impl osal_testkit::factory::TimerFactory for PosixTimerFactory {
    type Timer = PosixTimer;

    fn create_timer(
        &self,
        name: &str,
        period: Duration,
        mode: TimerMode,
        callback: TimerCallback,
    ) -> Result<Self::Timer> {
        PosixTimer::new(name, period, mode, callback)
    }
}

#[cfg(feature = "testkit")]
impl osal_testkit::factory::ClockFactory for PosixTimerFactory {
    type Clock = crate::clock::PosixClock;
}
