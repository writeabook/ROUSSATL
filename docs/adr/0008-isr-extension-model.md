# ADR 0008: ISR Extension Model

## Status

Accepted (2026-07-06)

## Context

P0 removed `isr_send`/`isr_recv` from the core `Queue` trait. P2 must
decide whether Semaphore follows the same pattern.

The current `CountingSemaphore` and `BinarySemaphore` traits include
`isr_acquire()` and `isr_release()` methods. However:

- POSIX has no true ISR context. ISR methods would be implemented as
  `acquire(Timeout::NoWait)`, which acquires an internal pthread mutex
  and is not guaranteed to be non-blocking.
- Mock has no interrupt model. ISR methods are trivial wrappers.
- FreeRTOS genuinely needs ISR-safe variants with `FromISR` suffix and
  `BaseType_t` return for `higher_priority_task_woken`.

Keeping ISR methods on core traits forces every backend to fake ISR
support, creating API-surface pollution and misleading capability
claims.

## Decision

ISR-safe operations are **removed** from the core `CountingSemaphore`
and `BinarySemaphore` traits during P2. This follows the same policy
established for `Queue` in P0.

Future FreeRTOS integration will introduce extension traits:

```rust
pub trait IsrSemaphore {
    fn acquire_from_isr(&self) -> Result<IsrWake>;
    fn release_from_isr(&self) -> Result<IsrWake>;
}
```

Where `IsrWake` indicates whether a higher-priority task was woken
and a context switch is needed.

## Rationale

- Consistent with P0's Queue decision (ADR 0003).
- Each backend only implements traits it can genuinely support.
- POSIX and Mock do not carry dead ISR code.
- Core traits stay minimal and correct for all current backends.
- Extension traits can be added without breaking existing code.

## Consequences

- `CountingSemaphore` loses `isr_acquire()` and `isr_release()`.
- `BinarySemaphore` loses `isr_acquire()` and `isr_release()`.
- Semaphore contract tests for ISR are removed.
- Behavior contract marks ISR semaphore operations as deferred.
- Mock and POSIX implementations do not include ISR stubs.

## Rejected Alternatives

### Keep ISR + return Unsupported on POSIX/Mock

Rejected because it clutters the API with methods that are known to
be unsupported on current backends. Users would need to handle
`Unsupported` for code that only targets POSIX.

### Keep ISR + implement via try-lock

Rejected because `acquire(NoWait)` still calls `pthread_mutex_lock`
internally, which is not ISR-safe. This creates a false sense of
safety.
