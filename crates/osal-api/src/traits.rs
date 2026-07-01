//! Trait definitions for OSAL abstractions.
//!
//! Each sub-module defines the contract that backend implementations
//! must fulfill. The traits are designed to be implementable across
//! diverse platforms: POSIX hosts, real-time kernels, and mock
//! environments.
//!
//! Trait modules will be populated in Phase 2.

// --- Synchronization primitives (Phase 2) ---
// pub mod mutex;
// pub mod semaphore;
// pub mod event_flags;

// --- Communication (Phase 2) ---
// pub mod queue;

// --- Execution (Phase 2) ---
// pub mod task;
// pub mod timer;

// --- System (Phase 2) ---
// pub mod clock;
// pub mod system;
