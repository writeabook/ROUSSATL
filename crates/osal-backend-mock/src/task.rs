//! Mock task implementation.
//!
//! Tasks execute synchronously in `spawn()` — the entry function runs
//! to completion before `spawn()` returns. This is sufficient for
//! deterministic contract smoke tests. A cooperative mock scheduler
//! is deferred to a later phase.
//!
//! # P6A: TLS and live count
//!
//! A backend-local `thread_local!` slot provides `current()` identity
//! during entry execution. A `LiveTaskToken` ensures `count()` reflects
//! running entries, not handle lifecycle.

use alloc::string::String;
use alloc::sync::Arc;
use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicUsize, Ordering};

use osal_api::error::{Error, Result};
use osal_api::time::Timeout;
use osal_api::traits::task::{Task, TaskBuilder};
use osal_api::types::{ExitCode, Priority, TaskHandle};

use osal_shared::validation;

// ---------------------------------------------------------------------------
// Global state
// ---------------------------------------------------------------------------

static NEXT_HANDLE: AtomicUsize = AtomicUsize::new(1);

/// Number of tasks whose entry function has not yet completed.
static LIVE_COUNT: AtomicUsize = AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// Current-task slot (single-threaded Mock — unsafeCell is sufficient)
// ---------------------------------------------------------------------------

struct CurrentSlot(UnsafeCell<Option<TaskHandle>>);
unsafe impl Sync for CurrentSlot {}

static CURRENT: CurrentSlot = CurrentSlot(UnsafeCell::new(None));

struct CurrentGuard {
    prev: Option<TaskHandle>,
}

impl CurrentGuard {
    fn enter(handle: TaskHandle) -> Self {
        // Safety: Mock tasks are single-threaded; no concurrent access.
        let prev = unsafe {
            let slot = &mut *CURRENT.0.get();
            let old = *slot;
            *slot = Some(handle);
            old
        };
        Self { prev }
    }
}

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        unsafe { *CURRENT.0.get() = self.prev; }
    }
}

// ---------------------------------------------------------------------------
// Live task token
// ---------------------------------------------------------------------------

/// RAII guard: increments `LIVE_COUNT` on creation, decrements on drop.
struct LiveTaskToken;

impl LiveTaskToken {
    fn register() -> Self {
        LIVE_COUNT.fetch_add(1, Ordering::SeqCst);
        Self
    }
}

impl Drop for LiveTaskToken {
    fn drop(&mut self) {
        LIVE_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

// ---------------------------------------------------------------------------
// Handle allocation
// ---------------------------------------------------------------------------

fn allocate_task_handle() -> Result<TaskHandle> {
    let raw = NEXT_HANDLE
        .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
            current.checked_add(1)
        })
        .map_err(|_| Error::Overflow)?;
    TaskHandle::from_raw(raw).ok_or(Error::Overflow)
}

// ---------------------------------------------------------------------------
// Inner — shared via Arc
// ---------------------------------------------------------------------------

struct MockTaskInner {
    handle: TaskHandle,
    priority: Priority,
    exit_code: ExitCode,
}

// ---------------------------------------------------------------------------
// MockTask
// ---------------------------------------------------------------------------

/// A mock task handle.
///
/// The entry function is executed synchronously in
/// [`MockTaskBuilder::spawn`]. `join()` returns the cached
/// `ExitCode::SUCCESS` immediately.
#[derive(Clone)]
pub struct MockTask {
    inner: Arc<MockTaskInner>,
}

impl Task for MockTask {
    fn join(&self, _timeout: Timeout) -> Result<ExitCode> {
        Ok(self.inner.exit_code)
    }

    fn handle(&self) -> TaskHandle {
        self.inner.handle
    }

    fn priority(&self) -> Priority {
        self.inner.priority
    }

    fn current() -> Option<TaskHandle> {
        // Safety: Mock tasks are single-threaded.
        unsafe { *CURRENT.0.get() }
    }

    fn count() -> usize {
        LIVE_COUNT.load(Ordering::SeqCst)
    }
}

// ---------------------------------------------------------------------------
// MockTaskBuilder
// ---------------------------------------------------------------------------

/// Builder for configuring and spawning a [`MockTask`].
pub struct MockTaskBuilder {
    name: String,
    stack_size: usize,
    priority: Priority,
}

impl TaskBuilder for MockTaskBuilder {
    type Task = MockTask;

    fn new() -> Self {
        Self {
            name: String::new(),
            stack_size: 4096,
            priority: 1,
        }
    }

    fn name(mut self, name: &str) -> Self {
        self.name.clear();
        self.name.push_str(name);
        self
    }

    fn stack_size(mut self, bytes: usize) -> Self {
        self.stack_size = bytes;
        self
    }

    fn priority(mut self, prio: Priority) -> Self {
        self.priority = prio;
        self
    }

    fn spawn<F>(self, entry: F) -> Result<Self::Task>
    where
        F: FnOnce() + Send + 'static,
    {
        validation::validate_task_config(&self.name, self.stack_size)?;

        let handle = allocate_task_handle()?;

        let inner = Arc::new(MockTaskInner {
            handle,
            priority: self.priority,
            exit_code: ExitCode::SUCCESS,
        });

        // Execute synchronously with correct TLS context and live count.
        {
            let _live = LiveTaskToken::register();
            let _context = CurrentGuard::enter(handle);
            entry();
        }

        Ok(MockTask { inner })
    }
}

// ---------------------------------------------------------------------------
// Factory (testkit)
// ---------------------------------------------------------------------------

#[cfg(feature = "testkit")]
pub struct MockTaskFactory;

#[cfg(feature = "testkit")]
impl osal_testkit::factory::TaskFactory for MockTaskFactory {
    type Task = MockTask;
    type TaskBuilder = MockTaskBuilder;

    fn task_builder(&self) -> Self::TaskBuilder {
        MockTaskBuilder::new()
    }
}
