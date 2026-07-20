//! Task trait — independent execution context.
//!
//! See [the behavior contract](../../../../docs/behavior-contract.md#8-task-contract)
//! for the full behavioral specification.

use crate::error::Result;
use crate::time::Timeout;
use crate::types::{ExitCode, Priority, TaskHandle};

/// An independent execution context (thread / RTOS task).
///
/// Tasks are created via a backend-specific builder and started with
/// [`spawn`](TaskBuilder::spawn). Once running, a task can be waited on
/// with [`join`](Task::join).
///
/// # Lifecycle
///
/// ```text
/// TaskBuilder::new() → .name(...) → .spawn(entry) → Task
///                                                     ↓
///                                                Running
///                                                     ↓
///                                           join() → ExitCode
/// ```
///
/// After a successful `spawn()`, the task may already be `Running` or
/// even `Finished`; portable code must not assume it is still `Ready`.
///
/// # Examples
///
/// ```ignore
/// use osal::prelude::*;
///
/// let task = TaskBuilder::new()
///     .name("worker")
///     .priority(2)
///     .spawn(|| { /* work */ })?;
///
/// let result = task.join(Timeout::Forever)?;
/// assert_eq!(result, ExitCode::SUCCESS);
/// ```
pub trait Task: Sized {
    /// Block until the task exits, or `timeout` expires.
    ///
    /// Returns `Ok(ExitCode)` on successful join. Once the task has
    /// exited, subsequent calls return the cached `ExitCode` immediately.
    ///
    /// Returns `Error::Timeout` if the task did not exit within
    /// `timeout`. The caller retains the handle and may retry.
    fn join(&self, timeout: Timeout) -> Result<ExitCode>;

    /// Return the opaque, non-zero handle identifying this task.
    fn handle(&self) -> TaskHandle;

    /// Return the task's configured priority.
    fn priority(&self) -> Priority;

    // ---- static methods ----

    /// Return the handle of the currently executing OSAL task.
    ///
    /// Returns `Some(TaskHandle)` when called from within an
    /// OSAL-created task's entry function. Returns `None` from the
    /// main thread or any non-OSAL context.
    fn current() -> Option<TaskHandle>;

    /// Return the number of OSAL tasks whose entry function has not
    /// yet completed.
    ///
    /// Finished tasks whose handle still exists are **not** counted.
    /// This is a snapshot for diagnostics only — do not use for
    /// concurrency synchronisation.
    fn count() -> usize;
}

// ---------------------------------------------------------------------------
// TaskBuilder
// ---------------------------------------------------------------------------

/// Builder for configuring and spawning a [`Task`].
///
/// Each backend provides a concrete `TaskBuilder` that produces that
/// backend's `Task` type.
///
/// # Defaults
///
/// | Field | Default |
/// |-------|---------|
/// | `name` | `""` |
/// | `stack_size` | `4096` (may be adjusted by backend) |
/// | `priority` | `1` |
///
/// # Examples
///
/// ```ignore
/// let task = TaskBuilder::new()
///     .name("sensor")
///     .stack_size(8192)
///     .priority(3)
///     .spawn(|| sensor_loop())?;
/// ```
pub trait TaskBuilder: Sized {
    /// The type of task this builder produces.
    type Task: Task;

    /// Create a new builder with default values.
    fn new() -> Self;

    /// Set the task name (informational, for debugging).
    ///
    /// Truncated to 31 bytes by the backend; must not contain embedded
    /// NUL bytes.
    fn name(self, name: &str) -> Self;

    /// Set the stack size in bytes.
    ///
    /// The backend enforces a minimum stack size; values below the
    /// minimum are clamped.
    fn stack_size(self, bytes: usize) -> Self;

    /// Set the task priority. Higher values = higher priority.
    fn priority(self, prio: Priority) -> Self;

    /// Consume the builder and start the task.
    ///
    /// The entry function `F` is executed exactly once in the new
    /// task context. It should not panic (panicking aborts the process
    /// on `panic=abort`).
    ///
    /// Returns `Error::InvalidParameter` if any builder field is out
    /// of range. Returns `Error::OutOfMemory` if the task cannot be
    /// allocated.
    fn spawn<F>(self, entry: F) -> Result<Self::Task>
    where
        F: FnOnce() + Send + 'static;
}
