//! Contract tests for POSIX task.
//!
//! Requires `--features testkit`:
//! ```bash
//! cargo test -p osal-backend-posix --features testkit
//! ```

use osal_backend_posix::runtime;
use osal_backend_posix::task::PosixTaskFactory;

#[test]
fn posix_task_core_contracts() {
    runtime::initialize().unwrap();
    let factory = PosixTaskFactory;
    osal_testkit::contract::task::run_core_contracts(&factory);
    runtime::shutdown().unwrap();
}
