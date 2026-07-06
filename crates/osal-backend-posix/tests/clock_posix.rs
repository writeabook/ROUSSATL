//! POSIX clock contract tests.
//!
//! Requires `--features testkit`:
//! ```bash
//! cargo test -p osal-backend-posix --features testkit
//! ```

use osal_backend_posix::timer::PosixTimerFactory;

#[test]
fn posix_clock_basic_contracts() {
    osal_testkit::contract::clock::run_basic_contracts(&PosixTimerFactory);
}

