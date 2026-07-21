# ADR 0018: POSIX Timer Service Lifecycle

## Status

Accepted (2026-07-21)

## Context

The current POSIX timer service uses `pthread_once` to lazy-initialise
a detached permanent worker thread.  This makes it impossible to stop,
join, or restart the service.  For the runtime lifecycle (ADR 0015),
backend services must support explicit initialise / shutdown /
re-initialise.

## Decision

### Lease exclusion for internal services

Timer Service, its worker thread, and its control block do **not**
hold `RuntimeLease`s.  During P6B-4, timer-service-local entry
liveness (the `timers` registry) prevents backend shutdown while
`Timer` handles remain alive.

`PosixTimer` handles will hold `RuntimeLease`s once facade runtime
integration is introduced (ADR 0015).  Until then, shutdown
protection is enforced at the timer-service level.

| Component | Holds RuntimeLease? |
|-----------|---------------------|
| `PosixTimer` handle | ❌ no (will hold in facade phase) |
| `TimerService` worker | ❌ no |
| `TimerServiceControl` block | ❌ no |

### Worker thread

- The worker thread is **joinable** (not detached).
- The service holds `Arc<TimerService>`.  The worker is given a clone
  via `Arc::into_raw` + `Arc::from_raw`.
- On shutdown, `stop_requested` is set, the condvar is broadcast, the
  worker exits, and `pthread_join` is called.
- The worker must not call `shutdown()` on itself (self-join detected
  via `pthread_equal` and returns `Error::Busy`).

### pthread_once for control block only

`pthread_once` may be used **only** for the permanent control block
(`TimerServiceControl` mutex + slot).  The service instance (timers,
worker thread) must be explicitly created and destroyed.  The control
block is process-lifetime; the service instance is runtime-lifetime.

### Control block vs. service instance

A process-lifetime control block holds a permanent mutex and slot.
The slot has three states:

- `Stopped` — no service instance
- `Running { service: Arc<TimerService>, worker: PosixThread, generation: u64 }` — active
- `Stopping { generation: u64 }` — shutdown in progress

The actual `TimerService` (timers, condvar, state) is created on
`initialize()` and destroyed on `shutdown()`.  The control block
persists across restarts.

### Lock ordering

```
Timer API:       control mutex → service mutex
shutdown:        control mutex → service mutex
worker loop:     only service mutex
callback:        holds neither lock
```

`service → control` is forbidden.

### Shutdown flow

```
control mutex
→ confirm Running
→ confirm not self-shutdown (pthread_equal)
→ service mutex
→ check no active timers (else Busy)
→ stop_requested = true
→ condvar.broadcast()
→ release service mutex
→ slot = Stopping
→ release control mutex
→ pthread_join worker
→ slot = Stopped
```

### Shutdown / re-initialise

| State     | `initialize()`              | `shutdown()`       |
|-----------|----------------------------|--------------------|
| `Stopped` | create service + worker    | `NotInitialized`   |
| `Running` | `AlreadyInitialized`       | stop + join worker |
| `Stopping`| `Busy`                      | `Busy`             |

`shutdown()` returns `Busy` while any live `Timer` handle exists
(checked under the service mutex after marking `stop_requested`).

### Timer API errors

All service functions (`register`, `start`, `stop`, `reset`,
`change_period`, `deregister`) return `Result` instead of silently
ignoring errors.  `PosixTimer` propagates these to the caller.
`Deregister` on drop uses `debug_assert!` (drop cannot return an
error).

| Error | Cause |
|-------|-------|
| `NotInitialized` | Service is `Stopped` |
| `Busy` | Service is `Stopping` |
| `NotFound` | Timer ID not found |
| `Overflow` | ID counter exhausted |
| `InvalidParameter` | Invalid period or argument |
| `Internal` | pthread failure |

### Callback execution

Callbacks execute outside the service mutex.  Callbacks must not
panic or unwind (`panic = "abort"` is the workspace default).  No
`catch_unwind` is used.

## Consequences

- `pthread_once` is only used for the permanent control block.
- The service instance is explicitly created and destroyed.
- Timer API signatures change from `-> ()` / `-> Option<u64>` to
  `-> Result<()>` / `-> Result<u64>`.
- `PosixTimer` propagates real errors instead of always returning
  `Ok(())`.
- Integration tests can stop and restart the timer service between
  test cases.
