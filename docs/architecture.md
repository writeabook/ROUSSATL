# OSAL Architecture

## 1. Overview

OSAL (Operating System Abstraction Layer) is a layered Rust framework for
building portable embedded and real-time applications. It allows you to
write application logic once and run it across different platforms —
POSIX hosts, real-time kernels, and mock environments — by changing a
single Cargo feature flag.

## 2. Design Goals

- **Portable applications**: Application code depends only on the `osal`
  facade crate. Switching platforms is a Cargo feature change, not a
  rewrite.
- **Backend independence**: Backend implementations are isolated behind
  public traits. Adding a new backend requires no changes to application
  code or to other backends.
- **Contract-driven quality**: Every backend must pass the same set of
  behavioral contract tests, ensuring consistent semantics across
  platforms.
- **Clean layering**: Each layer depends only on the layer below it.
  Platform details never leak into the public API.

## 3. Layer Architecture

```
Application
    ↓
osal (facade crate)
    ↓
osal-api (public traits and types)
    ↓
+-------+     +---------------+
| osal-  |     | osal-portable |
| shared |     | (helpers)     |
+-------+     +---------------+
    ↓               ↓
+-----------------------+
| osal-backend-*        |  ← platform-specific implementations
| (posix, freertos,     |
|  mock)                |
+-----------------------+
    ↓
osal-bsp + osal-bsp-*   ← board support packages
    ↓
Native OS / RTOS / hardware
```

### 3.1 `osal-api` — Foundation

The foundation crate. Defines **what** OSAL can do, not **how**.

- Public traits for all OS primitives (Mutex, Semaphore, Queue, Task,
  Timer, Clock, EventFlags, System)
- Shared types: `Error`, `Timeout`, `Result<T>`, `Handle`, `Priority`,
  `EventMask`, `StackSize`
- Zero runtime dependencies
- `no_std` compatible by default; optional `std` feature

Backend crates implement these traits. The `osal` facade re-exports
everything users need.

### 3.2 `osal-shared` — OS-Independent Logic

Shared implementation that all backends use:

- Object ID allocation and validation
- Resource registration and lookup tables
- Common parameter validation
- Initialization and lifecycle state management

Without this crate, each backend would reinvent object lifecycle logic,
leading to inconsistency.

### 3.3 `osal-portable` — Reusable Helpers

Utilities that multiple backends may optionally use:

- Ring buffer implementation
- Time conversion helpers
- Static memory pools for `no_std`
- Fallback no-op implementations for unsupported features

These are **internal building blocks**, not part of the public API.

### 3.4 `osal-backend-*` — Platform Implementations

Each backend crate implements all `osal-api` traits for a specific
platform:

| Crate | Platform | Use Case |
|-------|----------|----------|
| `osal-backend-posix` | Linux, macOS, POSIX | Development, CI, simulation |
| `osal-backend-mock` | In-process fake | Unit tests, contract verification |
| `osal-backend-freertos` | FreeRTOS | ARM Cortex-M, RISC-V embedded |

Backends depend on `osal-api`, `osal-shared`, and optionally
`osal-portable`. They must not depend on each other.

### 3.5 `osal-bsp` + `osal-bsp-*` — Board Support

Separates platform hardware configuration from OS backend logic:

- Boot and startup hooks
- Console / debug output
- Clock and timer hardware access
- Interrupt controller configuration
- Memory and heap region setup
- Resource limits (max tasks, max queues)

BSP crates sit below the OSAL layer and are selected independently
from the backend.

### 3.6 `osal-testkit` — Test Infrastructure

Shared testing utilities:

- Contract test harness for running behavior tests against any backend
- Assertion helpers for OSAL-specific verification
- Fake clock for deterministic timing
- Fault injection framework

### 3.7 `osal` — Facade

The only crate users depend on:

```toml
[dependencies]
osal = "0.1"
```

Responsibilities:
- Re-export `osal-api` types
- Select backend via Cargo features (`posix`, `mock`, `freertos`)
- Guard against multiple-backend selection at compile time
- Provide `prelude` module for convenient imports

## 4. Dependency Graph

```
osal-api  ←── osal-shared ←── osal-portable ←── osal-backend-posix
    ↑              ↑
    +── osal-bsp ←── osal-bsp-linux
    +── osal-testkit
    +── osal-backend-mock
    +── osal (facade)
```

No circular dependencies. Each crate depends only on crates below it.

## 5. Feature Flags

### 5.1 Facade-level features

```toml
[features]
default = ["posix"]        # POSIX backend by default
posix = ["osal-backend-posix"]
mock = ["osal-backend-mock"]
```

Rules:
- Exactly one backend must be selected at compile time
- `posix` is the default for development convenience
- `mock` is used for testing

### 5.2 Environment features

```toml
std = ["osal-api/std", "osal-shared/std"]
alloc = ["osal-api/alloc", "osal-shared/alloc"]
```

- `std`: Enables standard library (host test runners, examples)
- `alloc`: Enables heap allocation without full `std`

## 6. Naming Conventions

| Aspect | Convention | Example |
|--------|-----------|---------|
| Crate names | `osal-{layer}` | `osal-api`, `osal-backend-posix` |
| Trait names | Noun directly | `pub trait Mutex`, `pub trait Task` |
| Module files | `snake_case.rs` | `event_flags.rs`, `clock.rs` |
| Error type | `Error` (no lifetime parameter) | `Error::Timeout` |
| Return type | `Result<(), Error>` for boolean ops | `fn lock(&self) -> Result<()>` |
| ISR methods | `isr_` prefix | `isr_lock()`, `isr_signal()` |
| Backend types | Descriptive names | `Priority`, `EventMask`, `StackSize` |
| Prelude import | `use osal::prelude::*` | |
| Time types | `core::time::Duration` + `Timeout` enum | `Timeout::After(d)` |

## 7. Error Handling Strategy

OSAL uses a single, flat `Error` enum in `osal-api`:

```rust
pub enum Error {
    OutOfMemory,
    Timeout,
    QueueFull,
    QueueEmpty,
    LockFailed,
    NotFound,
    InvalidParameter,
    Unsupported,
    Internal(&'static str),
    // ...
}

pub type Result<T> = core::result::Result<T, Error>;
```

**No lifetime parameter** — keeps the type `Send + Sync + 'static`.

**Boolean-style operations** (lock, signal, wait) return
`Result<(), Error>` instead of a custom boolean type. This is more
idiomatic Rust and integrates with the `?` operator.

**Backend errors** (errno, FreeRTOS status codes) are mapped to OSAL
errors inside backend implementations. Raw platform error codes never
appear in the public API.

## 8. Module Organization Pattern

Within each crate, modules follow this pattern:

```
crates/osal-api/src/
├── lib.rs          # crate root, module declarations
├── error.rs        # Error enum and Result alias
├── time.rs         # Timeout, duration helpers
├── types.rs        # Common type aliases
├── traits.rs       # trait module declarations
├── traits/
│   ├── mutex.rs
│   ├── semaphore.rs
│   ├── queue.rs
│   ├── task.rs
│   ├── timer.rs
│   ├── clock.rs
│   ├── event_flags.rs
│   └── system.rs
└── prelude.rs      # selective re-exports
```

Backend crates mirror the trait structure with concrete implementations:

```
crates/osal-backend-posix/src/
├── lib.rs
├── task.rs
├── mutex.rs
├── semaphore.rs
├── queue.rs
├── timer.rs
├── clock.rs
└── sys/            # thin FFI wrappers
    ├── pthread.rs
    ├── condvar.rs
    ├── clock.rs
    └── errno.rs
```

## 9. Future Backends

To add a new backend:

1. Create `crates/osal-backend-{name}/` with `Cargo.toml` depending on
   `osal-api` + `osal-shared`
2. Implement all `osal-api` traits
3. Add the feature flag to `crates/osal/Cargo.toml`
4. Pass the contract test suite from `osal-testkit`

No changes to `osal-api`, `osal-shared`, or existing backends are
required.
