//! Monotonic time via `clock_gettime(CLOCK_MONOTONIC)`.

use core::time::Duration;

/// Return the current monotonic time as a `libc::timespec`.
pub fn monotonic_now_raw() -> libc::timespec {
    let mut ts: libc::timespec = unsafe { core::mem::zeroed() };
    unsafe {
        libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
    }
    ts
}

/// Return the current monotonic time as a `Duration`.
#[allow(dead_code)]
pub fn monotonic_now() -> Duration {
    let ts = monotonic_now_raw();
    Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}

/// Sleep for the given duration using `nanosleep`.
///
/// Uses `CLOCK_MONOTONIC` so it is unaffected by wall-clock changes.
pub fn nanosleep(d: Duration) {
    let req = libc::timespec {
        tv_sec: d.as_secs() as libc::time_t,
        tv_nsec: d.subsec_nanos() as libc::c_long,
    };
    unsafe {
        libc::nanosleep(&req, core::ptr::null_mut());
    }
}

/// Return `true` if `a >= b` (monotonic timespec comparison).
pub fn timespec_ge(a: &libc::timespec, b: &libc::timespec) -> bool {
    a.tv_sec > b.tv_sec || (a.tv_sec == b.tv_sec && a.tv_nsec >= b.tv_nsec)
}

