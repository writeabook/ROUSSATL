# ADR 0015: Runtime Lifecycle

## Status

Accepted (2026-07-20)

## Context

OSAL currently has no explicit initialisation or shutdown path.
Backend services such as the POSIX timer service initialise
lazily via `pthread_once` and run a detached permanent thread.
Objects can be created at any time without a guard. There is no
way to cleanly stop and restart the system — for example, between
integration tests or during a controlled system restart.

## Decision

### State machine

```
Uninitialized ── initialize() ──→ Initializing
     ↑                               │
     │                          success │ failure
     │                               ↓       ↓
     │                           Running   Uninitialized
     │                               │
     │                        shutdown()
     │                               │
     │                        ShuttingDown
     │                               │
     │                     success │     │ failure
     │                            ↓       ↓
     └─────────────────── Uninitialized   Running
```

No permanent `Terminated` state — this allows re-initialisation
within the same process (essential for integration tests).

### State transition rules

| Operation       | Current state   | Result |
|----------------|-----------------|--------|
| `initialize()` | `Uninitialized` | enter `Initializing` |
| `initialize()` | any other       | `Error::AlreadyInitialized` |
| `shutdown()`   | `Running`, no objects | enter `ShuttingDown` |
| `shutdown()`   | `Running`, objects alive | `Error::Busy` |
| `shutdown()`   | `Uninitialized` | `Error::NotInitialized` |
| Init failure   | `Initializing`  | rollback to `Uninitialized` |
| Shutdown fail  | `ShuttingDown`  | rollback to `Running` |
| Shutdown ok    | `ShuttingDown`  | transition to `Uninitialized` |

### Object lease tracking

Each OSAL object holds a `RuntimeLease` (RAII guard) that
increments an atomic active-object counter on creation and
decrements it on drop. Cloned handles share the same inner
state and do not create additional leases.

`shutdown()` checks the counter before proceeding. If non-zero,
it returns `Error::Busy`.

To prevent a race where an object is created just as shutdown
begins, `acquire_object()` uses a double-check pattern:

```rust
fn acquire_object(&self) -> Result<RuntimeLease> {
    if self.state() != RuntimeState::Running {
        return Err(Error::NotInitialized);
    }
    self.active_objects.fetch_add(1, Ordering::AcqRel);
    if self.state() != RuntimeState::Running {
        self.active_objects.fetch_sub(1, Ordering::AcqRel);
        return Err(Error::NotInitialized);
    }
    Ok(RuntimeLease { ... })
}
```

### New error variant

```rust
/// The runtime or resource is currently busy (objects still alive).
Busy,
```

This is a normal, expected error — not `Internal`.

### Object creation guards

Every constructor must:
1. Validate parameters first (error precedence: parameters >
   runtime state).
2. Acquire a runtime lease.
3. Create backend resources.

### Backend services

Backend runtime services (timer service, etc.) must support:
- Explicit `initialize()` — no lazy `pthread_once` for the
  service itself.
- Explicit `shutdown()` — stop thread, join, release resources.
- Re-initialisation after shutdown.

## Consequences

- `osal-shared` gains `runtime` module with `RuntimeLifecycle` and
  `RuntimeLease`.
- Every object inner struct gains a `_runtime: RuntimeLease` field.
- POSIX timer service is refactored from detached permanent thread
  to joinable owned service with explicit start/stop.
- All existing constructors gain a runtime lease acquisition step.
- `Error::Busy` added to `osal_api::error::Error`.
- Runtime contract tests verify initialisation, shutdown, lease,
  and error precedence across both backends.
