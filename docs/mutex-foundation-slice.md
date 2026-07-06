# Mutex Foundation Slice

## Status

Complete — Mutex vertical slice is implemented across the full stack:
API trait, Mock backend, POSIX backend, contract tests, facade, and
examples.

## Architecture

```
                 osal (facade)
                     |
         +-----------+-----------+
         |                       |
  osal-backend-posix    osal-backend-mock
         |                       |
  PosixMutexImpl<T>        MockMutex<T>
         |                       |
    PosixMutex              Rc + UnsafeCell
  (PTHREAD_MUTEX_         + Cell<usize>
   RECURSIVE)
```

## Components

| Layer | Type | Location |
|-------|------|----------|
| API | `Mutex<T>` trait | `crates/osal-api/src/traits/mutex.rs` |
| POSIX sys | `PosixMutex` (RECURSIVE) | `crates/osal-backend-posix/src/sys/mutex.rs` |
| POSIX backend | `PosixMutexImpl<T>` | `crates/osal-backend-posix/src/mutex.rs` |
| Mock backend | `MockMutex<T>` | `crates/osal-backend-mock/src/mutex.rs` |
| Facade | `Mutex` alias | `crates/osal/src/backend.rs` |
| Testkit | Mutex core contracts | `crates/osal-testkit/src/contract/mutex.rs` |
| Examples | mock_mutex, posix_mutex | `crates/osal/examples/` |

## Design Decisions

| Decision | Value |
|----------|-------|
| Recursive | Yes — same task can lock N times |
| Guard `!Send` | Yes — PhantomData<*const ()> |
| Guard drop | Only unlock path; no manual unlock |
| Poisoning | Not supported |
| NoWait failure | `Error::LockFailed` |
| After(ZERO) failure | `Error::Timeout` |
| POSIX type | `PTHREAD_MUTEX_RECURSIVE` |
| Mock model | `UnsafeCell<T>` + `Cell<usize>` (recursion counter) |

## Contract Tests Passing

### MutexCoreContract (Mock + POSIX)

8 tests:
- `create` — creation with initial value
- `lock_unlock` — uncontended lock, guard access, drop releases
- `guard_deref_mut` — mutable access via DerefMut
- `lock_forever` — Forever succeeds uncontended
- `lock_no_wait` — NoWait succeeds uncontended
- `recursive_lock` — same task locks twice
- `recursive_lock_three_levels` — three recursive locks, all see same data
- `guard_drop_releases_one_level` — inner drop still holds outer

### MutexBlockingContract (POSIX only)

3 tests:
- `no_wait_fails_when_held` — cross-thread NoWait → LockFailed
- `after_returns_timeout_when_held` — cross-thread After → Timeout
- `forever_woken_by_guard_drop` — cross-thread Forever → woken

## Intentionally Deferred

- Mock blocking/concurrency tests (single execution context; cross-task
  contention not simulated)
- ISR mutex operations (requires FreeRTOS extension trait)
- `close()` on Mutex (requires ADR; not part of current trait)

## Next Steps

1. Semaphore vertical slice (P2)
2. Task and Timer foundation slices
