# ADR 0014: Backend and BSP Responsibility Boundary

## Status

Accepted (2026-07-20)

## Context

The original OSAL architecture document described `osal-bsp` and
`osal-bsp-linux` as board support packages providing boot hooks,
console, clock, interrupt, memory, and resource configuration.
However, these crates currently contain only comment skeletons.
Several capabilities (`Clock`, `System::enter_critical()`, heap
reporting) have been implemented directly in backends without a
clear rule about which layer owns what.

Before adding runtime lifecycle or FreeRTOS support, the boundary
between backend and BSP must be explicitly defined.

## Decision

### Backend owns

- Task scheduling and thread creation
- Queue, Mutex, Semaphore, Timer
- OS timer service (background thread / ISR dispatch)
- Monotonic clock source (`Clock`)
- OS-level critical sections (`System::enter_critical()`)
- Native error code → `osal_api::Error` mapping
- Backend runtime service start / stop

### BSP owns

- Board / platform metadata (name, vendor, architecture)
- Boot and startup hooks
- Console / debug output
- Heap and memory region information source
- Static resource limits (max tasks, max queues, etc.)
- Chip-level or platform-level initialization
- Panic / fault hooks

### Things that stay in Backend (not BSP)

- `Clock`: monotonic time on POSIX comes from the OS; on FreeRTOS
  it comes from the RTOS tick. Not a board-level concern in MVP.
- `System::enter_critical()`: on POSIX this is a recursive mutex;
  on FreeRTOS this will be interrupt disable / BASEPRI. Neither is
  board-specific.

These capabilities may move to BSP later if concrete boards need
custom implementations, but that move should be a separate ADR.

### Dependency direction

```
Application → osal (facade)
                  ↓
             osal-api
                  ↓
        osal-shared + osal-portable
                  ↓
        osal-backend-posix / mock / freertos
                  ↓
        osal-bsp + osal-bsp-linux
                  ↓
        Native OS / RTOS / hardware
```

`osal-bsp` must not depend on `osal-api`. BSP traits return
BSP-specific types; the facade or backend maps them to OSAL types.

The existing `osal-bsp` dependency on `osal-api` is removed.

## Consequences

- `osal-bsp/Cargo.toml`: remove `osal-api` dependency.
- `Clock` and `System::enter_critical()` remain in backends.
- BSP crates are populated with concrete traits, not comment
  skeletons.
- `osal-bsp-linux` gets a minimal Linux BSP implementation.
- FreeRTOS backend will sit above FreeRTOS-compatible BSPs.
