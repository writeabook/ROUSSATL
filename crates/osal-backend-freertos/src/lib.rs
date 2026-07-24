//! FreeRTOS backend for the OSAL framework.
//!
//! Implements OSAL traits over a running FreeRTOS kernel.
//! The scheduler is owned by the application / BSP; this backend
//! is a guest of the kernel (ADR 0020).
//!
//! # Current status (P7C)
//!
//! **Implemented:**
//! - Runtime lifecycle (`initialize`, `shutdown`, `runtime_state`)
//! - Clock (`now` via coherent tick snapshots, `delay` with per-chunk guard ticks)
//! - System (`heap_free` via kernel heap, `enter_critical` with nesting)
//! - Mutex (native priority-inheritance mutex, RAII guard, non-recursive)
//! - CountingSemaphore (native kernel semaphore, count is sole source of truth)
//! - BinarySemaphore (native binary semaphore, initial count 0)
//!
//! **Deferred to P7D+:** Queue, Task, Timer, ISR extensions.

#![no_std]

extern crate alloc;

pub mod clock;
pub mod mutex;
pub mod runtime;
pub mod semaphore;
pub mod system;
pub(crate) mod wait;
