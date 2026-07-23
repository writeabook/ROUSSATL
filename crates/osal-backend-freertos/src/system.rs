//! FreeRTOS system — heap introspection and critical sections.
//!
//! Implements the OSAL [`System`] trait (ADR 0024).
//!
//! # Critical sections
//!
//! `taskENTER_CRITICAL()` / `taskEXIT_CRITICAL()` disable interrupts on
//! single-core configurations and support native nesting.  Each
//! `enter_critical()` call produces a new guard; dropping the outermost
//! guard fully re-enables interrupts.
//!
//! The guard is `!Send + !Sync` to prevent cross-task migration
//! (a guard entered on Task A must be dropped on Task A).
//!
//! # heap_free()
//!
//! `heap_free()` reports the FreeRTOS **kernel** heap free space via
//! `xPortGetFreeHeapSize()`.  This equals the Rust global allocator
//! free space only when the BSP maps the global allocator to
//! `pvPortMalloc` / `vPortFree`.
//!
//! # ISR context
//!
//! `enter_critical()` / `exit_critical()` are **not** callable from
//! ISR context.  ISR-safe critical sections (`taskENTER_CRITICAL_FROM_ISR`)
//! are deferred per ADR 0003 / ADR 0008.

use core::marker::PhantomData;
use alloc::rc::Rc;

use osal_api::traits::system::System;
use osal_backend_freertos_sys as sys;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// FreeRTOS system — heap introspection and critical-section entry.
///
/// All methods are associated functions (no `self`).  The system is a
/// process-wide singleton; the struct cannot be instantiated.
pub struct FreeRtosSystem;

/// RAII guard that exits the critical section on drop.
///
/// Created by [`FreeRtosSystem::enter_critical`].  The private field
/// prevents external construction (no way to obtain a guard without
/// entering a critical section).  `PhantomData<Rc<()>>` renders the
/// guard `!Send + !Sync`, preventing cross-task migration (ADR 0024 §3).
pub struct FreeRtosCriticalSectionGuard {
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for FreeRtosCriticalSectionGuard {
    fn drop(&mut self) {
        sys::exit_critical();
    }
}

// ---------------------------------------------------------------------------
// System trait
// ---------------------------------------------------------------------------

impl System for FreeRtosSystem {
    type CriticalSectionGuard = FreeRtosCriticalSectionGuard;

    fn heap_free() -> usize {
        usize::try_from(sys::heap_free()).unwrap_or(usize::MAX)
    }

    fn enter_critical() -> Self::CriticalSectionGuard {
        sys::enter_critical();
        FreeRtosCriticalSectionGuard {
            _not_send: PhantomData,
        }
    }
}

// ---------------------------------------------------------------------------
// Factory (testkit)
// ---------------------------------------------------------------------------

#[cfg(feature = "testkit")]
pub struct FreeRtosSystemFactory;

#[cfg(feature = "testkit")]
impl osal_testkit::factory::SystemFactory for FreeRtosSystemFactory {
    type System = FreeRtosSystem;
}

// ---------------------------------------------------------------------------
// Test-only helpers
// ---------------------------------------------------------------------------

/// Return the current critical-section nesting depth (fixture only).
#[cfg(feature = "testkit")]
pub fn critical_depth_for_test() -> usize {
    sys::critical_depth()
}
