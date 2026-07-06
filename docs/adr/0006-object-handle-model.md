# ADR 0006: Object Handle Model

## Status

Accepted (2026-07-06)

## Context

OSAL exposes OS resources (mutexes, queues, semaphores, timers, tasks)
through Rust types. There are multiple possible models for how these
Rust types relate to the underlying platform resources:

1. **Strong typed handles**: each Rust type wraps a backend resource; no
   global numeric ID registry.
2. **Global numeric ID table**: each object gets a numeric ID from a
   global registry; the Rust type is a thin wrapper around the ID.
3. **Hybrid**: typed handles internally, numeric IDs for C FFI.

These models have different implications for type safety, FFI
compatibility, runtime overhead, and API ergonomics.

## Decision

OSAL uses **strong typed handles** for all Rust public APIs.

- Each Rust type (`Mutex<T>`, `Queue`, `Semaphore`, `Timer`, `Task`)
  owns its backend resource through RAII.
- `Clone` on a handle creates another reference to the **same** backend
  resource (shared ownership via `Arc`/`Rc`).
- `Drop` on a handle releases only that reference; the backend resource
  is freed only when the last handle is dropped.
- **Guard types** (`MutexGuard<'a, T>`) borrow the handle and guarantee
  the protected resource remains valid for the guard's lifetime.
- No global numeric ID registry exists in the current phase.
- Backend-native types (`pthread_mutex_t`, `QueueHandle_t`) are never
  exposed in the public API.
- Future C ABI requirements can be met by adding a numeric ID adapter
  layer that wraps the typed handles, without changing the Rust API.

## Rationale

- **Type safety**: Strong types eliminate ID confusion bugs (passing
  a queue ID where a mutex ID is expected).
- **RAII determinism**: Drop semantics are clear and match Rust
  conventions.
- **No runtime overhead**: No global table lookups for operations on
  owned handles.
- **Evolvability**: A numeric ID layer can be added later for C FFI
  without breaking the Rust API. The typed handles are the canonical
  representation; numeric IDs are a derived compatibility layer.
- **Consistency with P0**: Queue, Mutex, Semaphore, Timer, and Task all
  follow the same handle model. This avoids per-type design divergence.

## Consequences

- No `id_map.rs`, `object_table.rs`, or global registry will be created
  during P1.
- Queue `Clone` semantics (multiple handles, same backend resource) are
  the template for Mutex and other types.
- Guard types must document their lifetime relationship to the parent
  handle — the handle must outlive the guard.
- Handle equality (two cloned handles are the same object) is enforced
  by the shared ownership model, not by numeric ID comparison.
- If C ABI is needed in the future, a separate `osal-ffi` crate can
  provide numeric ID wrappers around the typed handles.
