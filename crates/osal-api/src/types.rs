//! Common type aliases for the OSAL framework.
//!
//! These types abstract over platform-specific integer sizes, allowing
//! each backend to define its actual representation.

/// Opaque handle to an OSAL resource.
///
/// Handles are lightweight, copyable identifiers. The underlying value
/// is backend-defined; portable code should treat handles as opaque.
pub type Handle = usize;

/// Task/thread priority value. Higher values indicate higher priority.
pub type Priority = u32;

/// Set of event flags represented as a bitmask.
pub type EventMask = u32;

/// Stack size in bytes.
pub type StackSize = usize;
