//! POSIX clock — monotonic time via `clock_gettime(CLOCK_MONOTONIC)`.

use core::time::Duration;

use osal_api::traits::clock::Clock;

use crate::sys::time;

/// POSIX monotonic clock.
pub struct PosixClock;

impl Clock for PosixClock {
    fn now() -> Duration {
        let ts = time::monotonic_now_raw();
        Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
    }

    fn delay(duration: Duration) {
        time::nanosleep(duration);
    }
}
