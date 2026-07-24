//! Cross-thread blocking and wakeup tests.
//!
//! These verify that a blocked waiter is actually woken by a release
//! on another thread — not just "release before acquire" single-thread
//! ordering.
//!
//! All tests use `std::sync::Barrier` for deterministic synchronization
//! between the main thread and the worker thread.
//!
//! ```bash
//! cargo test -p osal-backend-freertos --features testkit sync_concurrent -- --test-threads=1
//! ```

#![cfg(feature = "testkit")]

use core::time::Duration;
use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, Barrier};
use std::thread;

use osal_api::error::Error;
use osal_api::time::Timeout;
use osal_api::traits::mutex::Mutex;
use osal_api::traits::semaphore::{BinarySemaphore, CountingSemaphore};
use osal_backend_freertos::mutex::FreeRtosMutex;
use osal_backend_freertos::runtime;
use osal_backend_freertos::semaphore::{FreeRtosBinarySemaphore, FreeRtosCountingSemaphore};
use osal_backend_freertos_sys::fixture;

fn setup() {
    fixture::reset();
    let _ = runtime::shutdown();
    runtime::initialize().expect("initialize");
}

fn teardown() {
    let _ = runtime::shutdown();
}

fn wait_for_waiter() {
    thread::sleep(Duration::from_millis(10));
}

// ---------------------------------------------------------------------------
// Mutex — cross-thread Forever wake
// ---------------------------------------------------------------------------

#[test]
fn mutex_forever_woken_by_cross_thread_guard_drop() {
    setup();

    let m = FreeRtosMutex::new(0u32).expect("create");
    let guard = m.lock(Timeout::NoWait).expect("lock");
    let m_clone = m.clone();
    let barrier = Arc::new(Barrier::new(2));
    let b = Arc::clone(&barrier);

    let handle = thread::spawn(move || {
        b.wait();
        let mut g = m_clone
            .lock(Timeout::Forever)
            .expect("worker should acquire");
        *g = 42;
        drop(g);
    });

    barrier.wait();
    wait_for_waiter();
    drop(guard);

    handle.join().expect("worker panicked");
    let g = m.lock(Timeout::NoWait).expect("lock");
    assert_eq!(*g, 42);

    teardown();
}

// ---------------------------------------------------------------------------
// CountingSemaphore — cross-thread Forever wake
// ---------------------------------------------------------------------------

#[test]
fn counting_semaphore_forever_woken_by_cross_thread_release() {
    setup();

    let s = FreeRtosCountingSemaphore::new(1, 0).expect("create");
    let s_clone = s.clone();
    let barrier = Arc::new(Barrier::new(2));
    let b = Arc::clone(&barrier);

    let handle = thread::spawn(move || {
        b.wait();
        s_clone
            .acquire(Timeout::Forever)
            .expect("worker should acquire");
    });

    barrier.wait();
    wait_for_waiter();
    s.release().expect("release");

    handle.join().expect("worker panicked");
    teardown();
}

// ---------------------------------------------------------------------------
// BinarySemaphore — cross-thread finite-wait early release
// ---------------------------------------------------------------------------

#[test]
fn binary_semaphore_after_early_release_wakes() {
    setup();

    let s = FreeRtosBinarySemaphore::new().expect("create");
    let s_clone = s.clone();
    let barrier = Arc::new(Barrier::new(2));
    let b = Arc::clone(&barrier);

    let handle = thread::spawn(move || {
        b.wait();
        s_clone
            .acquire(Timeout::After(Duration::from_millis(500)))
            .expect("worker should acquire, not timeout");
    });

    barrier.wait();
    wait_for_waiter();
    s.release().expect("release");

    handle.join().expect("worker panicked");
    teardown();
}

// ---------------------------------------------------------------------------
// Mutex — cross-thread After timeout
// ---------------------------------------------------------------------------

#[test]
fn mutex_held_cross_thread_after_returns_timeout() {
    setup();

    let m = FreeRtosMutex::new(0u32).expect("create");
    let _guard = m.lock(Timeout::NoWait).expect("lock");
    let m_clone = m.clone();
    let barrier = Arc::new(Barrier::new(2));
    let b = Arc::clone(&barrier);

    let handle = thread::spawn(move || {
        b.wait();
        let r = m_clone.lock(Timeout::After(Duration::from_millis(10)));
        r.map(drop)
    });

    barrier.wait();
    wait_for_waiter();

    let result = handle.join().expect("worker panicked");
    assert_eq!(result, Err(Error::Timeout));

    teardown();
}

// ---------------------------------------------------------------------------
// Mutex — NoWait from different thread fails
// ---------------------------------------------------------------------------

#[test]
fn mutex_held_cross_thread_nowait_returns_lock_failed() {
    setup();

    let m = FreeRtosMutex::new(0u32).expect("create");
    let _guard = m.lock(Timeout::NoWait).expect("lock");
    let m_clone = m.clone();

    let handle = thread::spawn(move || {
        let r = m_clone.lock(Timeout::NoWait);
        r.map(drop)
    });

    let result = handle.join().expect("worker panicked");
    assert_eq!(result, Err(Error::LockFailed));

    teardown();
}

// ---------------------------------------------------------------------------
// CountingSemaphore — one release wakes at most one waiter
// ---------------------------------------------------------------------------

#[test]
fn counting_semaphore_one_release_wakes_only_one_waiter() {
    setup();

    let s = FreeRtosCountingSemaphore::new(2, 0).expect("create");
    let b = Arc::new(Barrier::new(3));
    let done = Arc::new(AtomicUsize::new(0));

    let s1 = s.clone();
    let b1 = Arc::clone(&b);
    let d1 = Arc::clone(&done);
    let h1 = thread::spawn(move || {
        b1.wait();
        s1.acquire(Timeout::Forever).expect("waiter 1");
        d1.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });

    let s2 = s.clone();
    let b2 = Arc::clone(&b);
    let d2 = Arc::clone(&done);
    let h2 = thread::spawn(move || {
        b2.wait();
        s2.acquire(Timeout::Forever).expect("waiter 2");
        d2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    });

    // Wait for both workers to enter blocking.
    b.wait();
    thread::sleep(Duration::from_millis(20));

    // One release should wake exactly one waiter.
    s.release().expect("release");

    // Wait up to 2s for one waiter to complete.
    let mut acquired = 0;
    for _ in 0..200 {
        acquired = done.load(std::sync::atomic::Ordering::SeqCst);
        if acquired == 1 {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(acquired, 1, "exactly one waiter after one release");

    // Second release wakes the second waiter.
    s.release().expect("release 2");

    h1.join().expect("h1");
    h2.join().expect("h2");
    assert_eq!(done.load(std::sync::atomic::Ordering::SeqCst), 2);

    teardown();
}
