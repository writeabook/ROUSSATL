//! FreeRTOS clock — tick-based monotonic time via coherent kernel snapshots.
//!
//! Implements the OSAL [`Clock`] trait using `vTaskSetTimeOutState()`
//! (ADR 0023).  The tick counter may be 16-, 32-, or 64-bit depending
//! on the port configuration; the backend uses the kernel capability
//! probe to handle all widths uniformly.
//!
//! # Scheduler dependency
//!
//! - `now()` panics if the runtime has not been initialised (no cached
//!   capabilities) or the kernel returns an unexpected tick width.
//! - `delay(Duration::ZERO)` returns immediately in any scheduler state.
//! - `delay(d > 0)` requires the scheduler to be `Running` and the
//!   caller to be a FreeRTOS task.  It panics otherwise.

use core::time::Duration;

use osal_api::traits::clock::Clock;
use osal_portable::tick_time::{self, TickConfig, TickSnapshot};

use crate::runtime;
use osal_backend_freertos_sys as sys;

// ---------------------------------------------------------------------------
// Public type
// ---------------------------------------------------------------------------

/// FreeRTOS clock — monotonic tick counter backed by the kernel.
///
/// All methods are associated functions (no `self`).  The clock is a
/// process-wide singleton; the struct cannot be instantiated.
pub struct FreeRtosClock;

impl Clock for FreeRtosClock {
    fn now() -> Duration {
        let caps = runtime::capabilities()
            .expect("FreeRtosClock::now requires osal::initialize() to be called first");

        let snap = sys::tick_snapshot();

        let config = TickConfig {
            rate_hz: caps.tick_rate_hz,
            bits: caps.tick_bits,
        };

        // Convert the coherent snapshot to a Duration.  Saturates at
        // Duration::MAX rather than wrapping (ADR 0023 §2).
        tick_time::snapshot_to_duration(
            TickSnapshot {
                overflow_count: snap.overflow_count,
                tick_count: snap.tick_count,
            },
            config,
        )
        .expect("tick snapshot → Duration conversion failed (bad capability data)")
    }

    fn delay(duration: Duration) {
        // Zero delay — return immediately (ADR 0023 §6).
        if duration.is_zero() {
            return;
        }

        // Non-zero delay requires a Running scheduler (ADR 0023 §6).
        let state = sys::scheduler_state();
        if state != sys::SchedulerState::Running {
            panic!(
                "FreeRtosClock::delay requires a running scheduler \
                 and task context (scheduler state: {state:?})"
            );
        }

        let caps = runtime::capabilities()
            .expect("FreeRtosClock::delay requires osal::initialize() to be called first");

        // Ceiling conversion: non-zero duration → at least 1 tick.
        let ceil_ticks = tick_time::duration_to_ticks_ceil(duration, caps.tick_rate_hz)
            .expect("duration → ticks conversion overflowed");

        // Each vTaskDelay(n) call may lose up to nearly one tick of
        // wall-clock time because the remainder of the current tick
        // period is counted as the first "full" tick.  We add a guard
        // tick on every chunk, not just once globally (ADR 0023 §4).
        //
        // max_payload = max_finite - 1 reserves room for the guard tick
        // so that payload + 1 ≤ max_finite for every chunk.
        let max_native = sys::max_finite_delay_ticks() as u128;
        let max_payload = max_native
            .checked_sub(1)
            .expect("FreeRTOS max_finite_delay_ticks too small for guard tick");

        let mut remaining_payload = ceil_ticks;

        while remaining_payload > 0 {
            let payload = remaining_payload.min(max_payload);
            let native_ticks = payload.checked_add(1).expect("guard tick overflowed u128");

            let status = sys::delay_ticks(native_ticks as u64);
            if status != sys::DelayStatus::Ok {
                panic!(
                    "FreeRtosClock::delay failed: {status:?} \
                     (payload={payload}, native_ticks={native_ticks}, \
                      remaining_payload={remaining_payload})"
                );
            }

            remaining_payload -= payload;
        }
    }
}
