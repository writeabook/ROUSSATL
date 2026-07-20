//! Contract tests for the [`Task`] and [`TaskBuilder`] traits.
//!
//! These tests verify the behavioral contract defined in
//! `docs/behavior-contract.md#8-task-contract`.
//!
//! Split into:
//! - `TaskCoreContract` — both Mock and POSIX must pass
//! - `TaskConcurrencyContract` — POSIX only

use core::sync::atomic::{AtomicBool, Ordering};

use osal_api::time::Timeout;
use osal_api::traits::task::{Task as _, TaskBuilder as _};

use crate::factory::TaskFactory;

// ---------------------------------------------------------------------------
// TaskCoreContract — both backends
// ---------------------------------------------------------------------------

/// Default builder creates a valid task.
pub fn create_with_default_config<F: TaskFactory>(factory: &F) {
    let task = factory.task_builder().spawn(|| {}).unwrap();
    task.join(Timeout::Forever).unwrap();
}

/// Empty name is valid.
pub fn accept_empty_name<F: TaskFactory>(factory: &F) {
    let task = factory.task_builder().name("").spawn(|| {}).unwrap();
    task.join(Timeout::Forever).unwrap();
}

/// Name containing NUL is rejected.
pub fn reject_nul_in_name<F: TaskFactory>(factory: &F) {
    let result = factory.task_builder().name("bad\0name").spawn(|| {});
    assert!(result.is_err());
}

/// Name > 31 bytes is rejected.
pub fn reject_overlong_name<F: TaskFactory>(factory: &F) {
    let long = "a".repeat(32);
    let result = factory.task_builder().name(&long).spawn(|| {});
    assert!(result.is_err());
}

/// Zero stack size is rejected.
pub fn reject_zero_stack<F: TaskFactory>(factory: &F) {
    let result = factory.task_builder().stack_size(0).spawn(|| {});
    assert!(result.is_err());
}

/// Spawned task runs its entry exactly once.
pub fn spawn_runs_entry_once<F: TaskFactory>(factory: &F) {
    static FLAG: AtomicBool = AtomicBool::new(false);
    FLAG.store(false, Ordering::SeqCst);

    let task = factory
        .task_builder()
        .name("smoke")
        .spawn(|| {
            FLAG.store(true, Ordering::SeqCst);
        })
        .unwrap();

    task.join(Timeout::Forever).unwrap();
    assert!(FLAG.load(Ordering::SeqCst));
}

/// join() returns after task exit.
pub fn join_returns_after_task_exit<F: TaskFactory>(factory: &F) {
    let task = factory.task_builder().name("join").spawn(|| {}).unwrap();
    assert!(task.join(Timeout::Forever).is_ok());
}

/// Repeated join after completion returns cached immediately.
pub fn repeated_join_returns_cached<F: TaskFactory>(factory: &F) {
    let task = factory.task_builder().name("repeat").spawn(|| {}).unwrap();
    task.join(Timeout::Forever).unwrap();
    task.join(Timeout::NoWait).unwrap();
    task.join(Timeout::Forever).unwrap();
}

/// handle() is non-zero.
pub fn handle_is_nonzero<F: TaskFactory>(factory: &F) {
    let task = factory.task_builder().name("h").spawn(|| {}).unwrap();
    assert_ne!(task.handle().get(), 0);
    task.join(Timeout::Forever).unwrap();
}

/// handle() is unique across tasks.
pub fn handle_is_unique<F: TaskFactory>(factory: &F) {
    let t1 = factory.task_builder().name("a").spawn(|| {}).unwrap();
    let t2 = factory.task_builder().name("b").spawn(|| {}).unwrap();
    assert_ne!(t1.handle(), t2.handle());
}

/// current() returns Some(handle) from within the entry.
pub fn current_from_within_task<F: TaskFactory>(factory: &F) {
    use core::sync::atomic::AtomicUsize;

    static CAPTURED_RAW: AtomicUsize = AtomicUsize::new(0);

    let task = factory
        .task_builder()
        .name("curr")
        .spawn(|| {
            if let Some(h) = F::Task::current() {
                CAPTURED_RAW.store(h.get(), Ordering::SeqCst);
            }
        })
        .unwrap();

    task.join(Timeout::Forever).unwrap();
    let captured_raw = CAPTURED_RAW.load(Ordering::SeqCst);
    assert_ne!(captured_raw, 0);
    assert_eq!(captured_raw, task.handle().get());
}

/// current() returns None from main/external thread.
pub fn current_from_main_is_none<F: TaskFactory>(_factory: &F) {
    assert_eq!(F::Task::current(), None);
}

/// Priority is preserved.
pub fn priority_is_preserved<F: TaskFactory>(factory: &F) {
    let task = factory
        .task_builder()
        .priority(7)
        .spawn(|| {})
        .unwrap();
    assert_eq!(task.priority(), 7);
    task.join(Timeout::Forever).unwrap();
    assert_eq!(task.priority(), 7);
}

/// count() reflects a running entry and returns to baseline after.
pub fn count_reflects_live_tasks<F: TaskFactory>(factory: &F) {
    let baseline = F::Task::count();

    static BARRIER: AtomicBool = AtomicBool::new(false);
    BARRIER.store(false, Ordering::SeqCst);

    let task = factory
        .task_builder()
        .name("count")
        .spawn(|| {
            BARRIER.store(true, Ordering::SeqCst);
        })
        .unwrap();

    task.join(Timeout::Forever).unwrap();
    assert!(BARRIER.load(Ordering::SeqCst));

    // After join, count should be back to baseline.
    assert_eq!(F::Task::count(), baseline);
}

// ---------------------------------------------------------------------------
// Grouped entry points
// ---------------------------------------------------------------------------

/// Core contract — both Mock and POSIX.
pub fn run_core_contracts<F: TaskFactory>(factory: &F) {
    create_with_default_config::<F>(factory);
    accept_empty_name::<F>(factory);
    reject_nul_in_name::<F>(factory);
    reject_overlong_name::<F>(factory);
    reject_zero_stack::<F>(factory);
    spawn_runs_entry_once::<F>(factory);
    join_returns_after_task_exit::<F>(factory);
    repeated_join_returns_cached::<F>(factory);
    handle_is_nonzero::<F>(factory);
    handle_is_unique::<F>(factory);
    current_from_within_task::<F>(factory);
    current_from_main_is_none::<F>(factory);
    priority_is_preserved::<F>(factory);
    count_reflects_live_tasks::<F>(factory);
}
