//! Factory for creating task instances.

use osal_api::traits::task::{Task, TaskBuilder};

/// Factory for creating tasks in a backend-agnostic way.
pub trait TaskFactory {
    /// Concrete task type.
    type Task: Task;
    /// Concrete task builder type.
    type TaskBuilder: TaskBuilder<Task = Self::Task>;

    /// Create a new task builder with default configuration.
    fn task_builder(&self) -> Self::TaskBuilder;

    /// Hint that the scheduler should yield to other tasks.
    ///
    /// Used in concurrency tests. Backends without cooperative
    /// scheduling leave this as a no-op.
    fn yield_hint(&self) {}
}
