# ADR 0024: FreeRTOS System Mapping

## Status

Accepted (2026-07-25)

## Context

The OSAL `System` trait exports two capabilities: `heap_free()` returns
the free heap size, and `enter_critical()` returns a nesting-aware RAII
guard that provides mutual exclusion. FreeRTOS provides corresponding
native APIs, but the mapping involves semantic choices about which heap
is reported and the scope of the critical section.

## Decision

### 1. `heap_free()` → `xPortGetFreeHeapSize()`

```c
uint64_t osal_freertos_heap_free(void)
{
    return (uint64_t)xPortGetFreeHeapSize();
}
```

The Rust wrapper narrows to `usize`:

```rust
fn heap_free() -> usize {
    usize::try_from(sys::heap_free()).unwrap_or(usize::MAX)
}
```

`xPortGetFreeHeapSize()` reports the free space in the FreeRTOS kernel
heap (the one managed by `pvPortMalloc` / `vPortFree`). This is **not**
necessarily the same as the Rust global allocator's heap unless the BSP
configures the global allocator to delegate to `pvPortMalloc`.

The backend module documentation MUST state:

> `FreeRtosSystem::heap_free()` reports the FreeRTOS kernel heap.
> It equals the Rust global allocator free space only when the BSP
> maps the global allocator to `pvPortMalloc` / `vPortFree`.

### 2. Critical section → `taskENTER_CRITICAL()` / `taskEXIT_CRITICAL()`

```c
void osal_freertos_enter_critical(void)
{
    taskENTER_CRITICAL();
}

void osal_freertos_exit_critical(void)
{
    taskEXIT_CRITICAL();
}
```

FreeRTOS critical sections disable interrupts (on single-core) and
support nesting: each `taskENTER_CRITICAL()` increments a nesting
count, and `taskEXIT_CRITICAL()` decrements it. Interrupts are only
re-enabled when the count reaches zero. This matches the OSAL `System`
contract exactly: each `enter_critical()` call produces a new guard,
and the outermost drop fully releases the critical section.

### 3. Guard type: `!Send + !Sync`

```rust
use core::marker::PhantomData;
use alloc::rc::Rc;

pub struct FreeRtosCriticalSectionGuard {
    _not_send: PhantomData<Rc<()>>,
}

impl Drop for FreeRtosCriticalSectionGuard {
    fn drop(&mut self) {
        osal_backend_freertos_sys::exit_critical();
    }
}
```

`PhantomData<Rc<()>>` makes the guard both `!Send` and `!Sync`,
preventing:

- Moving a guard from Task A to Task B (`!Send`).
- Sharing a guard reference across threads (`!Sync`).

Without this, a guard entered on one task could be dropped on another,
corrupting the kernel's nesting counter and prematurely re-enabling
interrupts.

The private `_not_send` field also prevents external construction —
the only way to obtain a guard is via `FreeRtosSystem::enter_critical()`.

### 4. Guard Drop does not panic

`exit_critical()` does not allocate, does not block, and has no error
return. The `Drop` implementation is infallible.

### 5. Single-core constraint

```c
_Static_assert(
    configNUMBER_OF_CORES == 1,
    "P7B FreeRTOS backend currently supports single-core only"
);
```

FreeRTOS SMP (`configNUMBER_OF_CORES > 1`) requires spinlock-based
critical sections (`taskENTER_CRITICAL_FROM_ISR` / `portSET_INTERRUPT_MASK`)
with different semantics. SMP critical-section support is deferred to
a future phase.

### 6. ISR critical-section API deferred

FreeRTOS provides ISR-specific critical-section macros
(`taskENTER_CRITICAL_FROM_ISR` / `taskEXIT_CRITICAL_FROM_ISR`) that
return/set an interrupt mask rather than using a nesting counter.
A separate ISR-safe guard type with a distinct `System` method (or a
dedicated `IsrSystem` trait) is required for ISR context. This is
deferred per ADR 0003 and ADR 0008.

### 7. Task context constraint

`enter_critical()` is callable from any FreeRTOS task context while
the scheduler is running. It MUST NOT be called from ISR context
(see §6). It is callable before scheduler start (during
initialization), as `taskENTER_CRITICAL()` does not depend on the
scheduler — it only manipulates the interrupt mask.

### 8. System does not hold RuntimeLease

Per ADR 0019 §6, `Clock` and `System` are **stateless** primitives
that do not acquire a `RuntimeLease`. The backend runtime's
`acquire_object()` is only called by managed-object constructors
(Queue, Mutex, Semaphore, Task, Timer).

## Consequences

- `FreeRtosSystem::heap_free()` returns the FreeRTOS kernel heap free
  size, not necessarily the Rust allocator's free space.
- Critical sections use the native FreeRTOS interrupt-disable
  mechanism with full nesting support.
- The guard is `!Send + !Sync`, preventing cross-task migration.
- Guard Drop is infallible.
- Only single-core FreeRTOS configurations are supported in P7B.
- ISR critical-section entry requires a separate API, deferred to a
  future phase.
- System methods do not acquire a `RuntimeLease` and work before
  `osal::initialize()`.
