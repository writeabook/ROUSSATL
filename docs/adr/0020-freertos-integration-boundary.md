# ADR 0020: FreeRTOS Integration Boundary

## Status

Accepted (2026-07-25)

## Context

ROUSSATL currently has two active backends (POSIX, Mock) and a
defined but deferred FreeRTOS backend. Before implementing any
FreeRTOS primitives, the ownership boundary between the OSAL
backend, the BSP, the application, and the FreeRTOS kernel must
be explicitly defined. Without this boundary, each primitive
implementation risks embedding assumptions about scheduler
ownership, configuration provenance, and hardware setup that
would require crate restructuring later.

ADR 0014 defines the general Backend / BSP boundary. This ADR
specialises it for FreeRTOS specifically.

## Decision

### 1. Scheduler ownership

The FreeRTOS scheduler belongs to the **application / BSP**, not
to the OSAL backend.

- `osal::initialize()` initialises the OSAL runtime lifecycle and
  prepares the FreeRTOS backend for object creation. It does **not**
  call `vTaskStartScheduler()`.
- `osal::shutdown()` tears down the OSAL runtime lifecycle. It does
  **not** call `vTaskEndScheduler()`.
- The application or BSP is responsible for calling
  `vTaskStartScheduler()` at the appropriate point in its startup
  sequence, after hardware, interrupt, and tick configuration are
  complete.

Rationale: `vTaskStartScheduler()` transfers control to the RTOS;
the exact startup sequence (tick source, interrupt vectors, startup
task, hardware init) is platform-specific and belongs to the BSP
layer, not to a portable OSAL backend.

### 2. Kernel ownership

The FreeRTOS kernel source is **not** bundled with or compiled by
the OSAL backend crate.

```
Application / BSP
    ├── FreeRTOS Kernel source
    ├── FreeRTOSConfig.h
    ├── port.c / portASM
    ├── heap_x.c
    └── interrupt / startup code

ROUSSATL
    ├── osal-backend-freertos-sys   (C shim, capability probe)
    └── osal-backend-freertos       (safe Rust backend)
```

The backend crate depends only on a stable C ABI exposed through
a thin shim. It does not:
- Bundle a copy of the FreeRTOS kernel
- Provide a default `FreeRTOSConfig.h`
- Select a heap implementation (`heap_1`–`heap_5`)
- Select a port (Cortex-M, RISC-V, etc.)
- Require specific hardware or interrupt configuration

### 3. BSP responsibilities

The BSP or application **MUST** supply:

- `FreeRTOSConfig.h` with required configuration (see ADR 0021)
- A working FreeRTOS port layer
- A heap implementation supporting `pvPortMalloc` / `vPortFree`
- Tick source and interrupt configuration
- `vTaskStartScheduler()` invocation

### 4. Backend responsibilities

The FreeRTOS backend:

- Implements OSAL traits over the running FreeRTOS kernel
- Owns the OSAL `RuntimeLifecycle` instance (ADR 0019)
- Owns objects it creates (Queue, Mutex, Task, Timer, etc.)
- Maps FreeRTOS error codes to `osal_api::Error`
- Does **not** start or stop the scheduler
- Does **not** configure hardware or interrupts
- Does **not** own FreeRTOS kernel objects it did not create

### 5. Initialization order

```
Application startup
    → hardware / BSP init
    → FreeRTOS kernel init (implicit before scheduler start)
    → osal::initialize()
      → RuntimeLifecycle begin_initialize()
      → backend capability check
      → commit Running
    → create OSAL objects (Queue, Task, etc.)
    → vTaskStartScheduler()
```

### 6. Pre-scheduler object creation

Object creation (`Queue::new`, `TaskBuilder::spawn`, etc.) is
permitted before `vTaskStartScheduler()` is called, provided the
corresponding FreeRTOS API supports it. Many FreeRTOS object
creation functions work before the scheduler starts.

Blocking operations (`Queue::recv(Timeout::Forever)`,
`Semaphore::acquire(Timeout::Forever)`, `Task::join(Forever)`)
**MUST NOT** be called before the scheduler is running. The
backend may return `Error::NotInitialized` or `Error::Busy`
for these cases. The exact error is defined per-primitive during
implementation.

### 7. Shutdown semantics

- `osal::shutdown()` requires all OSAL objects to be dropped first
  (active-object count == 0, per RuntimeLease).
- Backend-internal resources are released.
- The FreeRTOS scheduler, if running, is **not** stopped.
- After shutdown, `osal::initialize()` may be called again
  (re-initialisation).

## Consequences

- The FreeRTOS backend is a **guest** of the running FreeRTOS
  kernel, not its owner.
- `osal::initialize()` / `osal::shutdown()` semantics are
  consistent with the existing POSIX and Mock backends (ADR 0015,
  ADR 0019).
- BSP crates (`osal-bsp`, `osal-bsp-linux`) remain deferred
  placeholders; the FreeRTOS BSP boundary is defined by this ADR
  but implemented in the application, not in a ROUSSATL crate.
- Blocking API semantics when the scheduler is not running must be
  defined per-primitive in the FreeRTOS backend implementation
  phase (P7B+).

## Deferred

- Static memory allocation (`xTaskCreateStatic`, etc.)
- ISR-safe APIs (`FromISR` variants)
- SMP / multi-core support
- MPU / memory protection
- Tickless idle
- Runtime scheduler state transitions (start → suspend → resume)
