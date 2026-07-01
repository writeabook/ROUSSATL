# OSAL Backend Contract

## 1. Purpose

This document defines the behavioral contract that every OSAL backend
must fulfill. It describes **what** correct behavior looks like, not
**how** to implement it.

Any crate implementing the `osal-api` traits must pass the contract
tests. This ensures that application code behaves consistently
regardless of which backend is active.

## 2. Backend Requirements

Every backend must:

1. Implement all traits in `osal-api` without panicking on valid inputs
2. Pass the contract test suite in `osal-testkit`
3. Map platform-specific errors to `osal_api::Error` variants
4. Document any intentional behavioral deviations
5. Not expose platform-specific types or error codes through the public
   API

## 3. Trait Contracts

### 3.1 Mutex

A mutual exclusion lock protecting a value of type `T`.

**Required behavior:**

- `lock(timeout)` acquires the lock, blocking up to `timeout`
- On success, returns a guard that provides `&mut T` access
- Dropping the guard releases the lock
- Recursive locking: the owning task may lock the same mutex multiple
  times (each `lock()` must be matched by one guard drop)
- Non-owning task must block or fail when attempting to lock an
  already-locked mutex
- `Timeout::NoWait` must return immediately; `Timeout::Forever` must
  block until acquired

**Error conditions:**
- `Error::LockFailed` — lock could not be acquired
- `Error::Timeout` — timeout expired before acquisition

**ISR behavior:**
- `isr_lock()` is non-blocking; returns `Error::LockFailed` if the
  mutex is held by another context

### 3.2 Semaphore

A counting semaphore for resource management and signaling.

**Required behavior:**

- `acquire(timeout)` decrements the count if > 0, otherwise blocks
- `release()` increments the count up to `max_count` and wakes one
  waiter
- `count()` returns the current count (non-blocking)
- `max_count()` returns the maximum count
- Binary semaphore is equivalent to `CountingSemaphore` with `max = 1`

**Error conditions:**
- `Error::Timeout` — timeout expired before acquiring
- `Error::InvalidParameter` — initial count > max count

**ISR behavior:**
- `isr_acquire()` is non-blocking
- `isr_release()` may be called from interrupt context

### 3.3 Queue

A bounded FIFO message queue for inter-task communication.

**Required behavior:**

- `send(msg, timeout)` enqueues a message; blocks if full
- `recv(msg, timeout)` dequeues a message; blocks if empty
- FIFO ordering: messages are received in the order they were sent
- Fixed message size: all messages have the same byte size
- Capacity is fixed at creation; `len()` / `capacity()` report state

**Error conditions:**
- `Error::Timeout` — send/recv timeout expired
- `Error::QueueFull` — non-blocking send on full queue
- `Error::QueueEmpty` — non-blocking recv on empty queue
- `Error::InvalidMessageSize` — message size does not match

**Wake-up rules:**
- A `send()` wakes at most one blocked receiver
- A `recv()` wakes at most one blocked sender

### 3.4 Task (Thread)

An independent execution context.

**Required behavior:**

- Tasks have a name, stack size, priority, and entry function
- `spawn()` starts the task; the entry function executes in the new
  context
- `join(timeout)` waits for task completion and returns the result
- Tasks are identified by an opaque `Handle`
- `current()` returns the handle of the calling task
- Priority determines scheduling order (higher = more urgent)

**Lifecycle states:**
- `Created` → `Ready` → `Running` → `Finished`
- Intermediate states: `Blocked`, `Suspended`

**Error conditions:**
- `Error::InvalidParameter` — name too long, stack too small
- `Error::AlreadyInitialized` — task already started
- `Error::NotInitialized` — attempting to join unstarted task

### 3.5 Timer

A software timer for delayed and periodic callbacks.

**Required behavior:**

- `start()` begins the countdown; callback executes after period
  elapses
- `stop()` prevents future callbacks (in-flight callbacks are not
  interrupted)
- `reset()` restarts the countdown from the current time
- `change_period(new_period)` updates the period for subsequent
  expirations
- One-shot: fires once and stops
- Periodic: automatically reloads after each expiration
- Callbacks execute outside the timer management lock

**Precision:**
- The timer must not expire before the period has elapsed
- Actual expiration may be later due to scheduling latency
- Real-time precision is backend-dependent

**Error conditions:**
- `Error::InvalidParameter` — period is zero

### 3.6 Event Flags

Multi-bit synchronization: tasks wait for specific bits to be set.

**Required behavior:**

- `set(bits)` sets the specified bits; wakes matching waiters
- `clear(bits)` clears the specified bits
- `get()` returns the current bitmask (non-blocking)
- `wait(mask, timeout)` blocks until **any** bit in `mask` is set
- Bits are **not** auto-cleared on return from `wait`
- The caller checks `returned & mask != 0` to determine success

**Wait semantics:**
- OR semantics (any bit): the default. Wait returns when any requested
  bit is set.

**Error conditions:**
- `Error::Timeout` — timeout expired with no matching bits set

### 3.7 Clock

Time measurement and delay primitives.

**Required behavior:**

- `now()` returns a monotonically increasing timestamp
- `elapsed(since)` returns the duration since a timestamp
- `delay(duration)` blocks the calling task for at least `duration`
- The clock must be monotonic; it must not go backward
- Tick period is backend-defined but must be documented

**Precision:**
- `delay(0)` must return immediately
- `delay(d)` must block for at least `d`; may block longer due to
  scheduling

### 3.8 System

Global system operations.

**Required behavior:**

- `critical_section_enter()` / `critical_section_exit()` provide mutual
  exclusion for short critical sections
- `heap_free()` returns available heap bytes (may return `usize::MAX`
  on virtual-memory systems)
- `task_count()` returns the number of registered tasks

**Critical section rules:**
- Critical sections may be nested
- Interrupts may be disabled on real-time backends
- On host systems, a process-local mutex is sufficient

## 4. ISR Safety

Some backends (FreeRTOS) distinguish between task context and interrupt
service routine (ISR) context. On backends without true ISRs (POSIX,
Mock), `isr_*` methods must:

- Be non-blocking
- Not wait on condition variables
- Complete in bounded time
- Return `Error::Unsupported` if the operation cannot be safely
  performed

## 5. Error Mapping

Each backend maps its native errors to `osal_api::Error`:

| Condition | OSAL Error |
|-----------|-----------|
| Memory allocation failure | `Error::OutOfMemory` |
| Timeout / deadline expired | `Error::Timeout` |
| Queue is full | `Error::QueueFull` |
| Queue is empty | `Error::QueueEmpty` |
| Lock contention | `Error::LockFailed` |
| Invalid argument | `Error::InvalidParameter` |
| Feature not available | `Error::Unsupported` |
| Unexpected native error | `Error::Internal("description")` |

Raw platform error codes (`errno`, FreeRTOS `pdFAIL`, etc.) must not
leak through the OSAL API.

## 6. Concurrency Guarantees

All public OSAL types must be `Send + Sync` where applicable.

- `Mutex<T>`: `Send + Sync` when `T: Send`
- `Queue`: `Send + Sync`
- `Semaphore`: `Send + Sync`
- `EventFlags`: `Send + Sync`
- `Task`: `Send + Sync`
- `Timer`: `Send + Sync`

Operations on these types are thread-safe by default. The `isr_*`
variants provide additional guarantees for interrupt context.

## 7. Contract Test Checklist

Each backend must pass these categories before acceptance:

**Mutex:**
- [ ] Create, lock, unlock
- [ ] Guard drop releases lock
- [ ] Recursive lock by owning task
- [ ] Cross-task mutual exclusion
- [ ] Non-blocking try-lock

**Semaphore:**
- [ ] Create with initial count
- [ ] Acquire decrements, release increments
- [ ] Timeout on empty
- [ ] Signal wakes waiting task
- [ ] Release at max count returns error

**Queue:**
- [ ] FIFO ordering
- [ ] Send succeeds when not full
- [ ] Recv succeeds when not empty
- [ ] Timeout on full/empty
- [ ] Blocked sender wakes after recv

**Task:**
- [ ] Create, spawn, join
- [ ] Pass parameters to entry function
- [ ] Multiple concurrent tasks
- [ ] Task metadata queries

**Timer:**
- [ ] One-shot fires once
- [ ] Periodic fires repeatedly
- [ ] Stop prevents callbacks
- [ ] Reset restarts countdown

**Event Flags:**
- [ ] Set, get, clear operations
- [ ] Wait returns when any bit set
- [ ] Wait times out on unset bits
- [ ] Bits not auto-cleared

**Clock / System:**
- [ ] Monotonic clock
- [ ] Delay blocks at least requested time
- [ ] Critical sections mutual exclusion
