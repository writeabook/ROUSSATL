# Changelog

## P1 — Mutex Vertical Slice (2026-07-06)

### Added

- ADR 0006: Object Handle Model (strong typed handles, no global ID registry).
- `Mutex<T>` backend implementations:
  - `MockMutex<T>` (`Rc` + `UnsafeCell` + `Cell<usize>`, recursive).
  - `PosixMutexImpl<T>` (`PTHREAD_MUTEX_RECURSIVE`, `try_lock`, `timed_lock`).
- `MutexCoreContract`: 8 tests covering creation, lock/unlock, recursive,
  guard semantics.
- `MutexBlockingContract`: 3 cross-thread tests (POSIX only).
- `mock_mutex` and `posix_mutex` examples.
- `docs/mutex-foundation-slice.md` — architecture, components, deferred items.

### Changed

- `sys/mutex.rs`: switched from `PTHREAD_MUTEX_ERRORCHECK` to
  `PTHREAD_MUTEX_RECURSIVE`.
- `sys/mutex.rs`: added `try_lock()` and `timed_lock()` methods.
- `behavior-contract.md`: fixed POSIX table (Mutex<T> → RECURSIVE);
  added timeout table, error mapping, non-requirements.
- README Mutex row updated from "API only" to fully implemented.
- `object-lifetime.md`: added Guard concept and four-layer object model.

### Fixed

- `docs/queue-foundation-slice.md`: removed "POSIX Queue implementation"
  from Intentionally Deferred; updated contract test counts; updated
  Status to Complete.

## P0 — Queue Vertical Slice Stabilization (2026-07-03)

### Added

- ADRs: error precedence, queue close semantics, ISR API policy, query
  method policy, mock runtime model.
- `Error::Overflow` now covers `capacity * msg_size` overflow.
- Error precedence rules: `InvalidMessageSize` > `QueueClosed` >
  `QueueFull`/`QueueEmpty` > `Timeout` > `Internal`.
- `After(Duration::ZERO)` semantics: resource available → success,
  unavailable → `Error::Timeout`.
- `ByteQueue::is_closed()` public accessor.
- Contract tests split into `QueueCoreContract` and `QueueBlockingContract`.
- Error precedence contract tests.
- CI workflow: format, clippy, test, docs, feature guards.

### Changed

- **`Queue::close()`**: return type `()` → `Result<()>`.
- **`Queue::len()`**: return type `usize` → `Result<usize>`.
- **`Queue::is_empty()`**: return type `bool` → `Result<bool>`.
- **`Queue::is_full()`**: return type `bool` → `Result<bool>`.
- `Queue::capacity()` and `Queue::msg_size()` are now documented as
  non-fallible (fixed at construction).
- `ByteQueue::new()` uses `checked_mul` and `try_reserve_exact` instead
  of the `vec![]` macro; returns `Error::Overflow` on overflow and
  `Error::OutOfMemory` on allocation failure.
- `ByteQueue::try_send()`: error precedence changed — checks
  `InvalidMessageSize` before `QueueClosed`.
- POSIX `QueueInner`: removed duplicate `closed` flag; uses
  `ByteQueue.is_closed()` as sole source.
- POSIX `QueueInner`: cached `capacity` and `message_size` for
  lock-free access.
- POSIX `send()`/`recv()`: no longer double-lock for size validation.
- POSIX `close()`: checks `is_closed()` for idempotency.
- Behavior contract: feature names unified to `backend-posix` /
  `backend-mock`.
- Behavior contract: Semaphore `release()` at max returns
  `Error::Overflow` instead of `Error::InvalidParameter`.
- Contract tests: all error assertions use precise `matches!` with
  exact variant.

### Removed

- **`Queue::isr_send()`** and **`Queue::isr_recv()`** removed from
  core `Queue` trait. ISR operations deferred to future `IsrQueue`
  extension trait.
- ISR methods removed from MockQueue and PosixQueue.
- ISR contract tests (`run_isr_contracts`) removed.
- Behavior contract: ISR descriptions removed from Queue and Mutex
  sections.
- Behavior contract: Mutex `isr_lock()` removed from contract doc
  (never existed in trait).
