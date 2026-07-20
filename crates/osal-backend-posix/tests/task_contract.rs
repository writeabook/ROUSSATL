//! Contract tests for POSIX task.
//!
//! Requires `--features testkit`:
//! ```bash
//! cargo test -p osal-backend-posix --features testkit
//! ```

use osal_backend_posix::task::PosixTaskFactory;

#[test]
fn posix_task_core_contracts() {
    let factory = PosixTaskFactory;
    osal_testkit::contract::task::run_core_contracts(&factory);
}
