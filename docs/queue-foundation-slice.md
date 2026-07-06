# Queue Foundation Slice

## Status

Complete — Queue vertical slice is stabilized and frozen (p0-complete).
Both POSIX and Mock backends pass the full contract test suite. CI
enforces format, clippy, tests, docs, and feature matrix checks.

## Architecture

```
                 osal (facade)
                     |
         +-----------+-----------+
         |                       |
  osal-backend-posix    osal-backend-mock
         |                       |
    PosixQueue              MockQueue
         |                       |
    ByteQueue +            ByteQueue +
    condvar/mutex          Rc<RefCell<>>
```

## Components

| Layer | Type | Location |
|-------|------|----------|
| API | `Queue` trait | `crates/osal-api/src/traits/queue.rs` |
| Portable | `ByteQueue` | `crates/osal-portable/src/byte_queue.rs` |
| Shared | `validate_queue_*` | `crates/osal-shared/src/validation.rs` |
| POSIX | `PosixQueue` | `crates/osal-backend-posix/src/queue.rs` |
| Mock | `MockQueue` | `crates/osal-backend-mock/src/queue.rs` |
| Mock | `MockFaultFactory` | `crates/osal-backend-mock/src/fault.rs` |
| Facade | `Queue` alias | `crates/osal/src/backend.rs` |
| Testkit | Queue core contracts | `crates/osal-testkit/src/contract/queue/` |
| Testkit | Clone lifetime contracts | `crates/osal-testkit/src/contract/lifetime.rs` |

## Contract Tests Passing

### QueueCoreContract (Mock + POSIX)

- `creation::run` — 3 tests (valid create, reject zero capacity, reject zero msg_size)
- `fifo::run` — 4 tests (roundtrip, FIFO order, send full→QueueFull, recv empty→QueueEmpty)
- `error_precedence::run` — 4 tests (wrong send size, wrong recv size, closed+wrong send→InvalidMessageSize, closed+wrong recv→InvalidMessageSize)
- `close::run` — 5 tests (send after close→QueueClosed, recv empty after close→QueueClosed, drain after close, close idempotent, metadata after close)
- `timeout::run` — 2 tests (send timeout on full, recv timeout on empty)

Total: 18 core contract tests across all backends.

### QueueBlockingContract (POSIX only)

- `recv_forever_woken_by_send` — 1 test
- `send_forever_woken_by_recv` — 1 test
- `recv_after_returns_timeout` — 1 test
- `send_after_returns_timeout_when_full` — 1 test
- `close_wakes_blocked_recv` — 1 test
- `close_wakes_blocked_send` — 1 test

Total: 6 blocking contract tests (POSIX only).

### Additional

- `lifetime::run_clone_contracts` — 3 tests (clone shares state, drop clone keeps alive, close affects all clones)
- `fault::run_queue_fault_contracts` — 3 tests (Mock only)

## Intentionally Deferred

- ISR queue operations (requires `IsrQueue` extension trait; deferred to FreeRTOS phase)
- Mock blocking scheduler emulation (Mock returns `Error::Unsupported` for `Timeout::Forever` on full/empty)

## Next Steps

1. Mutex vertical slice (P1) — follow same pattern: contract → mock → posix → facade → CI
2. Semaphore vertical slice (after Mutex)
3. Task and Timer foundation slices
