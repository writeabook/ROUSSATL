//! Contract tests for POSIX timer.
//!
//! POSIX passes core contracts.
//!
//! Requires `--features testkit`:
//! ```bash
//! cargo test -p osal-backend-posix --features testkit
//! ```

use osal_backend_posix::timer::PosixTimerFactory;
use osal_backend_posix::timer_service;

#[test]
fn posix_timer_core_contracts() {
    timer_service::initialize().unwrap();
    let factory = PosixTimerFactory;
    osal_testkit::contract::timer::run_core_contracts(&factory);
    timer_service::shutdown().unwrap();
}
