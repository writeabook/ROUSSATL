//! Contract tests for mock timer.
//!
//! Mock passes core + controlled contracts.
//!
//! Requires `--features testkit`:
//! ```bash
//! cargo test -p osal-backend-mock --features testkit -- --test-threads=1
//! ```

use osal_backend_mock::clock;
use osal_backend_mock::timer::MockTimerFactory;

#[test]
fn mock_timer_core_contracts() {
    clock::init_runtime();
    let factory = MockTimerFactory;
    osal_testkit::contract::timer::run_core_contracts(&factory);
}

#[test]
fn mock_timer_controlled_contracts() {
    clock::init_runtime();
    let factory = MockTimerFactory;
    osal_testkit::contract::timer::run_controlled_contracts(&factory);
}
