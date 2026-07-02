//! Testing infrastructure for OSAL backends.
//!
//! Provides:
//!
//! - **[`BackendFactory`]** — trait for creating primitives in a
//!   backend-agnostic way.
//! - **[`contract`]** — generic contract test functions that run
//!   against any backend implementing `BackendFactory`.
//! - **[`assertions`]** — no-std-compatible assertion macros.

#![no_std]

pub mod assertions;
pub mod contract;
pub mod factory;
