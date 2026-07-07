//! Contract tests for POSIX system.
//!
//! Requires `--features testkit`:
//! ```bash
//! cargo test -p osal-backend-posix --features testkit
//! ```

use osal_backend_posix::system::PosixSystemFactory;

#[test]
fn posix_system_contracts() {
    let factory = PosixSystemFactory;
    osal_testkit::contract::system::run_all(&factory);
}
