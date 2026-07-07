//! Contract tests for mock system.
//!
//! Requires `--features testkit`:
//! ```bash
//! cargo test -p osal-backend-mock --features testkit
//! ```

use osal_backend_mock::system::MockSystemFactory;

#[test]
fn mock_system_contracts() {
    let factory = MockSystemFactory;
    osal_testkit::contract::system::run_all(&factory);
}
