# ADR 0023: FreeRTOS Tick and Time Model

## Status

Accepted (2026-07-25)

## Context

The OSAL `Clock` trait requires a monotonic time source that returns
`Duration` and a `delay` that blocks for at least the requested
`Duration`. FreeRTOS provides a tick-based time model: the kernel
increments a `TickType_t` counter on each tick interrupt, and tasks
block via `vTaskDelay()` for a requested number of ticks.

Directly exposing `xTaskGetTickCount()` has two problems:

1. `TickType_t` wraps around (16-bit, 32-bit, or 64-bit depending on
   port configuration). Without overflow tracking, `now()` would
   appear to jump backwards after ~49.7 days at 1 kHz with 32-bit ticks.
2. `vTaskDelay()` blocks for a number of ticks, but the actual wall-clock
   delay depends on where in the current tick period the call occurs.
   A naive `ceil(duration × rate_hz)` conversion can return earlier than
   the contract requires.

This ADR defines how FreeRTOS kernel ticks are converted into
`Duration` and how `delay()` guarantees "at least d."

## Decision

### 1. Time source: coherent kernel snapshot

`Clock::now()` uses `vTaskSetTimeOutState()` to capture a consistent
snapshot of both the native tick count and the kernel's overflow count:

```c
typedef struct {
    uint64_t overflow_count;
    uint64_t tick_count;
} osal_freertos_tick_snapshot_t;

osal_freertos_tick_snapshot_t osal_freertos_tick_snapshot(void)
{
    TimeOut_t native;
    vTaskSetTimeOutState(&native);

    osal_freertos_tick_snapshot_t result;
    result.overflow_count = (uint64_t)(UBaseType_t)native.xOverflowCount;
    result.tick_count     = (uint64_t)native.xTimeOnEntering;
    return result;
}
```

`vTaskSetTimeOutState()` reads `xTickCount` and `xNumOfOverflows`
atomically inside a critical section, so the Rust side never sees a
half-updated pair.

The expanded tick is computed in Rust as:

```rust
let total_ticks: u128 =
    ((snapshot.overflow_count as u128) << tick_bits)
    | snapshot.tick_count as u128;
```

This formula is correct for 16-, 32-, and 64-bit `TickType_t` without
Rust knowing the native layout of `TimeOut_t`.

### 2. Tick → Duration conversion

Given `total_ticks: u128` and `rate_hz: u32`:

```text
seconds   = total_ticks / rate_hz
remainder = total_ticks % rate_hz
nanos     = (remainder × 1_000_000_000) / rate_hz
```

All intermediate arithmetic uses `u128`. The result is a `Duration`.

If the computed seconds exceed `Duration::MAX` (roughly 584 billion
years), the result saturates to `Duration::MAX` rather than wrapping.
`Clock::now()` has no `Result` return type, so saturation preserves
the "never decreases" contract.

### 3. Duration → tick conversion (ceiling)

```text
ticks = ceil(d.as_nanos() × rate_hz / 1_000_000_000)
```

All non-zero durations round **up** to at least 1 tick. Rounding down
would convert a sub-tick duration into 0 ticks, causing `delay()` to
return early in violation of the contract.

`Duration::ZERO` maps to 0 ticks.

### 4. Guard tick for delay

`Clock::delay(d)` computes:

```text
delay_ticks = ceil_ticks + 1
```

The extra guard tick compensates for the fact that `vTaskDelay(n)`
blocks for *up to* `n` full tick periods, but the call can occur
anywhere within the current tick. Adding one tick ensures the caller
is blocked for **at least** the requested `Duration` regardless of
phase alignment.

FreeRTOS documentation confirms that `vTaskDelay()` blocks for the
requested number of tick interrupts *from the next tick*, not from
the moment of the call.

### 5. Long delay chunking

The native delay parameter is `TickType_t`. To keep the C ABI stable,
the shim accepts `uint64_t`:

```c
uint32_t osal_freertos_delay_ticks(uint64_t ticks);
uint64_t osal_freertos_max_finite_delay_ticks(void);
```

`osal_freertos_max_finite_delay_ticks()` returns `portMAX_DELAY - 1`.
The Rust backend splits delays that exceed this maximum into
consecutive finite chunks:

```rust
while remaining_ticks > 0 {
    let chunk = remaining_ticks.min(max_finite_ticks as u128);
    sys::delay_ticks(chunk as u64);
    remaining_ticks -= chunk;
}
```

`portMAX_DELAY` is reserved as the "forever" sentinel; it is never
used as an ordinary finite wait.

### 6. Scheduler state contract

- `delay(Duration::ZERO)` returns immediately in **any** scheduler
  state. No tick inspection occurs.
- `delay(d > 0)` requires the scheduler to be `Running` and the
  caller to be a FreeRTOS task. If the scheduler is `NotStarted` or
  `Suspended`, the backend panics with a descriptive message:

  ```rust
  panic!(
      "FreeRtosClock::delay requires a running scheduler \
       and task context"
  );
  ```

  Silent return is **not** allowed because it would violate the
  "at least d" contract. Busy-looping in the backend is not an
  option because the tick counter does not advance before scheduler
  start without BSP-specific hardware timer knowledge.

### 7. ISR context

`Clock::now()` and `Clock::delay()` are **not** callable from ISR
context in P7B. ISR-safe clock access is deferred to a future phase
(ADR 0003, ADR 0008).

### 8. Scheduler restart

FreeRTOS `xTickCount` resets to 0 on `vTaskStartScheduler()`, and
`xNumOfOverflows` resets to 0. If the scheduler is stopped and
restarted, `Clock::now()` may jump backwards. This is consistent with
the contract: the clock is monotonic *while initialized*, and a
scheduler restart is a new initialization epoch.

## Consequences

- `Clock::now()` never decreases during a scheduler run.
- Tick wrap does not cause clock regression, regardless of
  `TickType_t` width.
- All tick↔Duration conversions use checked arithmetic; overflow
  produces `Error::Overflow` where a `Result` is available, or
  saturates to `Duration::MAX` where it is not.
- Non-zero `delay()` never returns early due to tick rounding.
- Very long delays are split into chunks compatible with the native
  `TickType_t` width.
- Scheduler-state violations are caught immediately with a clear
  panic message, not silently ignored.
- The `osal-portable` tick conversion module is backend-neutral and
  can be reused for other tick-based RTOS backends.
