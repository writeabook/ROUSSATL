//! POSIX Timer Service — single joinable pthread for timer callbacks.
//!
//! Managed through `timer_control::TimerServiceControl` (ADR 0018).
//! The service instance is explicitly created on `initialize()` and
//! destroyed on `shutdown()`.  The worker thread is joinable and holds
//! an `Arc<TimerService>` for its lifetime.
//!
//! # Lock ordering
//!
//! ```text
//! Timer API:       control mutex → service mutex
//! shutdown:        control mutex → service mutex
//! worker loop:     only service mutex
//! callback:        holds neither lock
//! ```

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::UnsafeCell;
use core::ffi::c_void;
use core::time::Duration;

use osal_api::traits::timer::TimerCallback;
use osal_portable::timer_state::TimerState;

use crate::sys::condvar::PosixCondvar;
use crate::sys::mutex::{PosixMutex, PosixMutexGuard};
use crate::sys::task::PosixThread;
use crate::sys::time;
use crate::timer_control::{self, ServiceSlot};

// ---------------------------------------------------------------------------
// Entry
// ---------------------------------------------------------------------------

struct TimerEntry {
    id: u64,
    state: TimerState,
    callback: Option<TimerCallback>,
    deleted: bool,
}

// ---------------------------------------------------------------------------
// Service state
// ---------------------------------------------------------------------------

pub(crate) struct TimerServiceState {
    timers: Vec<TimerEntry>,
    next_id: u64,
    stop_requested: bool,
}

// ---------------------------------------------------------------------------
// Service instance
// ---------------------------------------------------------------------------

pub(crate) struct TimerService {
    mutex: PosixMutex,
    condvar: PosixCondvar,
    state: UnsafeCell<TimerServiceState>,
}

impl TimerService {
    fn new() -> Result<Self, ()> {
        Ok(Self {
            mutex: PosixMutex::new().map_err(|_| ())?,
            condvar: PosixCondvar::new().map_err(|_| ())?,
            state: UnsafeCell::new(TimerServiceState {
                timers: Vec::new(),
                next_id: 1,
                stop_requested: false,
            }),
        })
    }

    fn with_state_locked<R>(
        &self,
        _guard: &PosixMutexGuard<'_>,
        f: impl FnOnce(&mut TimerServiceState) -> R,
    ) -> R {
        unsafe { f(&mut *self.state.get()) }
    }

    /// Worker loop.  Returns when `stop_requested` is set.
    fn run(&self) {
        loop {
            let mut guard = self.mutex.lock_guard().unwrap();

            if self.with_state_locked(&guard, |s| s.stop_requested) {
                return;
            }

            let state = unsafe { &mut *self.state.get() };
            state.timers.retain(|e| !e.deleted);

            let now = time::monotonic_now();
            let mut earliest: Option<Duration> = None;
            for e in &state.timers {
                if let Some(d) = e.state.deadline() {
                    match earliest {
                        None => earliest = Some(d),
                        Some(cur) if d < cur => earliest = Some(d),
                        _ => {}
                    }
                }
            }

            match earliest {
                None => {
                    let _ = self.condvar.wait(&mut guard);
                }
                Some(deadline) if deadline <= now => {
                    drop(guard);
                    self.dispatch_one();
                }
                Some(deadline) => {
                    let timeout = deadline.saturating_sub(now);
                    let abs = time::abs_deadline(timeout);
                    let _ = self.condvar.timed_wait(&mut guard, &abs);
                }
            }
        }
    }

    /// Dispatch ONE expired callback.  Callback executes outside all locks.
    fn dispatch_one(&self) {
        let (id, mut callback) = {
            let _guard = self.mutex.lock_guard().unwrap();
            let state = unsafe { &mut *self.state.get() };
            let now = time::monotonic_now();

            let mut best_idx: Option<usize> = None;
            for (i, e) in state.timers.iter().enumerate() {
                if e.deleted || e.callback.is_none() {
                    continue;
                }
                if let Some(d) = e.state.deadline() {
                    if d <= now {
                        best_idx = Some(i);
                        break;
                    }
                }
            }

            let Some(idx) = best_idx else { return };
            let entry = &mut state.timers[idx];
            if !entry.state.advance_on_expiry(now) {
                return;
            }
            (entry.id, entry.callback.take().unwrap())
        };

        callback();

        // Re-acquire and restore callback.
        let _guard = self.mutex.lock_guard().unwrap();
        let state = unsafe { &mut *self.state.get() };
        if let Some(entry) = state.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            if entry.callback.is_none() {
                entry.callback = Some(callback);
            }
        }
    }
}

unsafe impl Send for TimerService {}
unsafe impl Sync for TimerService {}

// ---------------------------------------------------------------------------
// Worker entry point
// ---------------------------------------------------------------------------

extern "C" fn timer_worker(arg: *mut c_void) -> *mut c_void {
    let service = unsafe { Arc::from_raw(arg.cast::<TimerService>()) };
    service.run();
    core::ptr::null_mut()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Initialise the timer service.
///
/// Returns `AlreadyInitialized` if already running, `Busy` if
/// shutdown is in progress.
pub fn initialize() -> Result<(), osal_api::error::Error> {
    use osal_api::error::Error;

    timer_control::with_control(|ctrl| match &ctrl.slot {
        ServiceSlot::Stopped => {
            let service = Arc::new(TimerService::new().map_err(|_| Error::Internal("timer service creation failed"))?);
            let worker_ref = Arc::into_raw(Arc::clone(&service)).cast_mut().cast::<c_void>();
            let cfg = crate::sys::task::PosixThreadConfig { stack_size: 4096 };
            let worker = PosixThread::spawn(&cfg, timer_worker, worker_ref)
                .map_err(|_| {
                    unsafe { drop(Arc::from_raw(worker_ref.cast::<TimerService>())); }
                    Error::Internal("timer worker spawn failed")
                })?;
            let generation = ctrl.next_generation;
            ctrl.next_generation += 1;
            ctrl.slot = ServiceSlot::Running { service, worker, generation };
            Ok(())
        }
        ServiceSlot::Running { .. } => Err(Error::AlreadyInitialized),
        ServiceSlot::Stopping { .. } => Err(Error::Busy),
    })
}

/// Shut down the timer service.
///
/// Waits for in-flight callbacks to complete, joins the worker
/// thread, and transitions back to `Stopped`.  The `pthread_join`
/// is done while holding the control mutex — this is safe because
/// the worker loop only touches the service mutex (lock ordering
/// control → service is preserved).
pub fn shutdown() -> Result<(), osal_api::error::Error> {
    use osal_api::error::Error;

    timer_control::with_control(|ctrl| {
        let (service, generation) = match &ctrl.slot {
            ServiceSlot::Stopped => return Err(Error::NotInitialized),
            ServiceSlot::Stopping { .. } => return Err(Error::Busy),
            ServiceSlot::Running {
                service, generation, ..
            } => (Arc::clone(service), *generation),
        };

        // Signal the worker to exit.
        {
            let _guard = service.mutex.lock_guard().unwrap();
            let state = unsafe { &mut *service.state.get() };
            state.stop_requested = true;
            let _ = service.condvar.broadcast();
        }

        // Replace the slot to extract the worker handle.
        let old = core::mem::replace(&mut ctrl.slot, ServiceSlot::Stopping { generation });
        let mut worker = match old {
            ServiceSlot::Running { worker, .. } => worker,
            _ => unreachable!(),
        };

        // Join the worker.  The control lock is still held but the
        // worker only accesses the service mutex — no deadlock.
        worker
            .try_join()
            .expect("timer worker join failed — internal invariant violated");

        ctrl.slot = ServiceSlot::Stopped;
        Ok(())
    })
}

/// Returns `true` if the timer service is initialised and running.
pub fn is_running() -> bool {
    timer_control::with_control(|ctrl| matches!(ctrl.slot, ServiceSlot::Running { .. }))
}

// ---------------------------------------------------------------------------
// Timer operations — these go through the control block to find the
// current service instance.
// ---------------------------------------------------------------------------

fn with_service<R>(f: impl FnOnce(&TimerService, &mut TimerServiceState) -> R) -> Option<R> {
    timer_control::with_control(|ctrl| match &ctrl.slot {
        ServiceSlot::Running { service, .. } => {
            let guard = service.mutex.lock_guard().ok()?;
            let state = unsafe { &mut *service.state.get() };
            let result = f(service, state);
            drop(guard);
            Some(result)
        }
        _ => None,
    })
}

pub fn register(
    period: Duration,
    mode: osal_api::types::TimerMode,
    callback: TimerCallback,
) -> Option<u64> {
    with_service(|svc, state| {
        let id = state.next_id;
        state.next_id = state.next_id.wrapping_add(1);
        if id == 0 {
            // Skip 0 — reserved as invalid.
            state.next_id = 1;
        }
        let id = if id == 0 { 1 } else { id };
        state.timers.push(TimerEntry {
            id,
            state: TimerState::new(period, mode).expect("period validated by caller"),
            callback: Some(callback),
            deleted: false,
        });
        let _ = svc.condvar.signal();
        id
    })
}

pub fn start(id: u64) {
    with_service(|svc, state| {
        let now = time::monotonic_now();
        if let Some(e) = state.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            let _ = e.state.start(now);
            let _ = svc.condvar.signal();
        }
    });
}

pub fn stop(id: u64) {
    with_service(|svc, state| {
        if let Some(e) = state.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            e.state.stop();
            let _ = svc.condvar.signal();
        }
    });
}

pub fn reset(id: u64) {
    with_service(|svc, state| {
        let now = time::monotonic_now();
        if let Some(e) = state.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            let _ = e.state.reset(now);
            let _ = svc.condvar.signal();
        }
    });
}

pub fn change_period(id: u64, new_period: Duration) {
    with_service(|svc, state| {
        if let Some(e) = state.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            let _ = e.state.change_period(new_period);
            let _ = svc.condvar.signal();
        }
    });
}

pub fn deregister(id: u64) {
    with_service(|svc, state| {
        if let Some(e) = state.timers.iter_mut().find(|e| e.id == id && !e.deleted) {
            e.deleted = true;
            e.state.stop();
            e.callback = None;
            let _ = svc.condvar.signal();
        }
    });
}
