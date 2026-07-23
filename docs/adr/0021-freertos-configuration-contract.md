# ADR 0021: FreeRTOS Configuration Contract

## Status

Accepted (2026-07-25)

## Context

FreeRTOS is configured at compile time through `FreeRTOSConfig.h`
macros. The OSAL backend needs to know certain configuration values
(e.g. tick rate, max priorities, whether dynamic allocation is
enabled) to correctly size types, validate parameters, and decide
which code paths are available.

The backend must not guess these values, embed a default config,
or depend on the raw FreeRTOS headers from Rust.

## Decision

### 1. Required configuration

The application **MUST** provide a `FreeRTOSConfig.h` with at least:

```c
#define configSUPPORT_DYNAMIC_ALLOCATION 1
#define INCLUDE_xTaskGetSchedulerState    1
#define configUSE_TIMERS                  1
```

The following macros **MUST** be defined and have valid,
non-zero values:

```c
configTICK_RATE_HZ
configMAX_PRIORITIES
configMAX_TASK_NAME_LEN
```

The C shim (`osal_freertos_shim.c`) reads these at compile time
and exposes them through a stable C ABI. The Rust backend queries
capabilities through the `-sys` crate, never by parsing
`FreeRTOSConfig.h` directly.

### 2. Capability probe

`osal-backend-freertos-sys` exposes a single probe function:

```c
osal_freertos_capability_t osal_freertos_probe_capabilities(void);
```

returning a struct with:

| Field | Source | Type |
|-------|--------|------|
| `tick_rate_hz` | `configTICK_RATE_HZ` | `uint32_t` |
| `max_priorities` | `configMAX_PRIORITIES` | `uint32_t` |
| `max_task_name_len` | `configMAX_TASK_NAME_LEN` | `uint32_t` |
| `tick_bits` | `sizeof(TickType_t) * 8` | `uint8_t` |
| `stack_word_size` | `sizeof(StackType_t)` | `uint8_t` |
| `dynamic_allocation` | `configSUPPORT_DYNAMIC_ALLOCATION` | `uint8_t` (bool) |
| `software_timers` | `configUSE_TIMERS` | `uint8_t` (bool) |
| `scheduler_state` | `xTaskGetSchedulerState()` | `uint32_t` |

The Rust backend calls this once during `initialize()` and caches
the result. Public OSAL APIs never expose raw FreeRTOS macros.

### 3. Missing configuration → compile error

If a required macro is not defined or has an invalid value, the
shim **MUST** emit a `#error` directive at C compile time:

```c
#ifndef configSUPPORT_DYNAMIC_ALLOCATION
#error "configSUPPORT_DYNAMIC_ALLOCATION must be defined"
#endif
#if configSUPPORT_DYNAMIC_ALLOCATION != 1
#error "configSUPPORT_DYNAMIC_ALLOCATION must be 1 for the current OSAL backend"
#endif
```

The Rust backend does not perform runtime capability checks for
required features — violations are caught at C compile time.

### 4. Optional capabilities

Capabilities that may vary between valid configurations (e.g.
`configUSE_16_BIT_TICKS`) are exposed through the capability struct
but do not cause compile errors. The Rust backend may degrade
gracefully (e.g. narrower tick range) or return `Error::Unsupported`
for features that require a specific configuration.

### 5. Tick width

The first supported configuration uses `TickType_t` = 32-bit
(`configUSE_16_BIT_TICKS == 0`). The shim reports `tick_bits` from
`sizeof(TickType_t)`. The Rust backend scales its internal
arithmetic to the reported width.

16-bit and 64-bit `TickType_t` are **not** validated in the initial
implementation but the probe field and conversion code are designed
to accommodate them.

## Consequences

- `osal-backend-freertos-sys` owns the capability probe.
- `osal-backend-freertos` caches `KernelCapabilities` at init time.
- Missing required configuration is a C compile-time error, not a
  Rust runtime error.
- The Rust backend never includes or parses `FreeRTOSConfig.h`.
- Public OSAL APIs contain no FreeRTOS-specific types or macros.
- Configuration changes require recompiling the C shim (and
  therefore the Rust `-sys` crate).
