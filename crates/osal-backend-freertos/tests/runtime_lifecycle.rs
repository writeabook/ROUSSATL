//! FreeRTOS backend runtime lifecycle tests.
//!
//! Uses the test-fixture capability probe (no real FreeRTOS kernel
//! required).  Verifies the same lifecycle contract as POSIX and Mock.
//!
//! ```bash
//! cargo test -p osal-backend-freertos --features testkit -- --test-threads=1
//! ```

use osal_api::error::Error;
use osal_api::runtime::RuntimeState;
use osal_backend_freertos::runtime;

// ---------------------------------------------------------------------------
// Basic state transitions
// ---------------------------------------------------------------------------

#[test]
fn initial_state_is_uninitialized() {
    // Clean start.
    let _ = runtime::initialize();
    runtime::shutdown().ok();
    assert_eq!(runtime::state(), RuntimeState::Uninitialized);
}

#[test]
fn initialize_enters_running() {
    runtime::initialize().unwrap();
    assert_eq!(runtime::state(), RuntimeState::Running);
    runtime::shutdown().unwrap();
}

#[test]
fn repeated_initialize_returns_already_initialized() {
    runtime::initialize().unwrap();
    let result = runtime::initialize();
    assert_eq!(result, Err(Error::AlreadyInitialized));
    assert_eq!(runtime::state(), RuntimeState::Running);
    runtime::shutdown().unwrap();
}

#[test]
fn shutdown_returns_to_uninitialized() {
    runtime::initialize().unwrap();
    runtime::shutdown().unwrap();
    assert_eq!(runtime::state(), RuntimeState::Uninitialized);
}

#[test]
fn shutdown_before_initialize_returns_not_initialized() {
    runtime::initialize().unwrap();
    runtime::shutdown().unwrap();
    let result = runtime::shutdown();
    assert_eq!(result, Err(Error::NotInitialized));
}

#[test]
fn runtime_can_reinitialize() {
    runtime::initialize().unwrap();
    runtime::shutdown().unwrap();
    runtime::initialize().unwrap();
    assert_eq!(runtime::state(), RuntimeState::Running);
    runtime::shutdown().unwrap();
}

// ---------------------------------------------------------------------------
// active_objects (testkit)
// ---------------------------------------------------------------------------

#[test]
fn no_active_objects_when_idle() {
    runtime::initialize().unwrap();
    assert_eq!(runtime::active_objects(), 0);
    runtime::shutdown().unwrap();
}

// ---------------------------------------------------------------------------
// Busy shutdown with live object
// ---------------------------------------------------------------------------

#[test]
fn live_object_blocks_shutdown() {
    runtime::initialize().unwrap();
    let _lease = osal_backend_freertos::runtime::acquire_object_for_test();
    let result = runtime::shutdown();
    assert_eq!(result, Err(Error::Busy));
    // Drop lease, shutdown should now succeed.
    drop(_lease);
    runtime::shutdown().unwrap();
}
