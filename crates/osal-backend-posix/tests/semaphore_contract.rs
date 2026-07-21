//! Contract tests for POSIX semaphores.
//!
//! Requires `--features testkit`:
//! ```bash
//! cargo test -p osal-backend-posix --features testkit
//! ```

use osal_backend_posix::runtime;
use osal_backend_posix::semaphore::PosixSemaphoreFactory;

#[test]
fn posix_semaphore_core_contracts() {
    runtime::initialize().unwrap();
    let factory = PosixSemaphoreFactory;
    osal_testkit::contract::semaphore::run_all(&factory);
    runtime::shutdown().unwrap();
}
