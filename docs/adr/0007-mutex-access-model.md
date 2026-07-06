# ADR 0007: Mutex Access Model

## Status

Accepted (2026-07-06)

## Context

P1 delivered a recursive `Mutex<T>` where each `lock()` call returns a
`MutexGuard` implementing `DerefMut<Target = T>`. While recursive
locking is convenient for some patterns, the combination of recursion
and `DerefMut` is unsound in Rust:

```rust
let mut g1 = mutex.lock(Timeout::Forever).unwrap();
let mut g2 = mutex.lock(Timeout::Forever).unwrap();
*g1 = 1;  // &mut T
*g2 = 2;  // &mut T — aliased mutable reference!
```

This violates Rust's fundamental aliasing rule: at most one `&mut T` to
a given location at any time. The existence of two simultaneously live
`&mut T` references is undefined behavior, even if the underlying
pthread mutex prevents data races at the OS level.

## Decision

`Mutex<T>` is changed to **non-recursive**:

- Only one `MutexGuard<T>` can exist at a time.
- Re-locking the mutex from the same context while a guard is alive
  returns `Error::LockFailed` (for `NoWait`) or `Error::Timeout` (for
  `After(ZERO)`).
- `MutexGuard` retains `Deref<Target = T>` and `DerefMut<Target = T>`.
- Recursive locking capability is deferred to a future `RecursiveMutex`
  type with a separate API design.

## Rationale

1. **Safety by construction**: A non-recursive mutex with a single
   mutable guard cannot produce aliased `&mut T`. The Rust compiler's
   aliasing rules are satisfied.
2. **Standard Rust convention**: `std::sync::Mutex` does not support
   recursive locking. This is the expected default.
3. **Simpler to verify**: Non-recursive locking eliminates the need to
   track recursion depth in backends.
4. **Recursive locking is rare in practice**: Most embedded and systems
   code uses non-recursive mutexes. When recursive locking is genuinely
   needed, it should be an explicit opt-in via a distinct type.

## Consequences

- **API**: No trait signature changes. `Mutex<T>` and `MutexGuard<'a, T>`
  are unchanged. Only the behavioral contract changes.
- **Mock**: `MockMutex<T>` replaces recursion counter with a boolean
  `locked` flag. `lock()` when already locked returns `LockFailed`.
- **POSIX**: `PTHREAD_MUTEX_RECURSIVE` → `PTHREAD_MUTEX_ERRORCHECK`.
  Same-task re-lock returns `EDEADLK` → `Error::Internal`.
- **Contract tests**: Recursive tests removed. New test:
  `no_second_guard` — verifies second lock fails.
- **Documentation**: Trait docs, behavior contract updated.
- **No regression**: Existing non-recursive users are unaffected.
  Recursive users must redesign — their code was unsound.

## Rejected Alternatives

### Keep recursive + remove DerefMut

Only provide `Deref<Target = T>` (read-only) on nested guards. Rejected
because it changes the API contract for all users, not just recursive
ones, and makes the common case (single guard, mutate) less ergonomic.

### Keep recursive + runtime panic

Panic on re-lock when a guard already exists. Rejected because panics
are not recoverable in embedded/RTOS contexts and violate the OSAL
error-return contract.

### Keep recursive + internal mutability only

Require users to wrap `T` in `Cell`/`RefCell` for mutation. Rejected
because it pushes the safety burden onto users and makes the API more
complex for the common non-recursive case.
