# ADR 0011: System Critical Section Model

## Status

Accepted (2026-07-07)

## Context

P4 System Foundation Slice implements `System::enter_critical()` which
must support nested entry and RAII guard-based exit. Two backends exist
with fundamentally different execution models:

- **Mock** runs deterministically in a single execution context;
  thread-level mutual exclusion is unnecessary.
- **POSIX** runs with real threads; nested critical sections need a
  recursive lock to avoid self-deadlock.

Additionally, the public `Mutex<T>` trait is non-recursive (ADR 0007).
If `System::enter_critical()` reused the same non-recursive mutex,
nested calls would deadlock.

## Decision

1. **Mock** uses an `AtomicUsize` nesting counter. `enter_critical()`
   increments it; the guard's `Drop` decrements it. No real mutex is
   held — the counter validates the nesting contract for deterministic
   tests.

2. **POSIX** uses a dedicated process-local recursive
   `pthread_mutex_t` (`PTHREAD_MUTEX_RECURSIVE`), initialised once via
   `pthread_once`. This is a separate wrapper (`sys::recursive_mutex`)
   from the non-recursive `sys::mutex::PosixMutex` used by `Mutex<T>`.

3. **Guard constructibility**: both `MockCriticalSectionGuard` and
   `PosixCriticalSectionGuard` carry a private `_private: ()` field.
   External code cannot construct a guard directly. This prevents:
   - counter underflow on Mock (drop without matching
     `enter_critical()`)
   - `pthread_mutex_unlock` on an unheld mutex on POSIX

4. **`heap_free()`** returns `usize::MAX` for both backends in MVP.
   Real heap introspection is deferred to the BSP/resource phase.

## Rationale

- Recursive POSIX mutex avoids deadlock on nested
  `enter_critical()`.
- Dedicated `sys::recursive_mutex` keeps the non-recursive
  `sys::mutex::PosixMutex` clean for `Mutex<T>`.
- Mock atomic counter is sufficient for contract test validation;
  real mutex semantics would add complexity without benefit.
- Private guard fields close a soundness hole without affecting
  the public trait contract.
- `usize::MAX` default follows the `System` trait documentation.

## Consequences

- Two new files: `sys/recursive_mutex.rs` and `system.rs` per backend.
- POSIX critical sections use `pthread_once` for lazy initialisation.
- Mock critical sections do not provide thread-level mutual exclusion
  (consistent with the single-context Mock model).
- Real heap introspection is deferred to BSP phase.
- System contract tests validate nesting, re-entry, and guard-drop
  ordering across both backends.
