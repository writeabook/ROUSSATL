//! Contract tests for mock task.
//!
//! Requires `--features testkit`:
//! ```bash
//! cargo test -p osal-backend-mock --features testkit
//! ```

use osal_backend_mock::runtime;
use osal_backend_mock::task::MockTaskFactory;

#[test]
fn mock_task_core_contracts() {
    runtime::initialize().unwrap();
    let factory = MockTaskFactory;
    osal_testkit::contract::task::run_core_contracts(&factory);
    runtime::shutdown().unwrap();
}
