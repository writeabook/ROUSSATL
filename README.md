# OSAL — Operating System Abstraction Layer

A portable, layered abstraction over real-time and general-purpose operating
systems for Rust applications.

## Overview

OSAL provides a unified API for developing multi-platform embedded and
real-time applications. Write your application logic once and run it
across different platforms by switching the backend.

## Supported Backends

| Backend | Status | Target |
|---------|--------|--------|
| POSIX   | Planned | Linux, macOS, CI |
| Mock    | Planned | Unit tests, simulation |
| FreeRTOS | Planned | ARM Cortex-M, RISC-V |

## Architecture

OSAL uses a layered crate architecture:

```
Application
    ↓
osal (facade)          ← what you depend on
    ↓
osal-api               ← public traits and types
    ↓
osal-shared + osal-portable  ← shared logic and helpers
    ↓
osal-backend-*         ← platform-specific implementations
    ↓
osal-bsp + osal-bsp-*  ← board support packages
```

See [docs/architecture.md](docs/architecture.md) for details.

## Quick Start

```toml
[dependencies]
osal = "0.1"
```

```rust
use osal::prelude::*;
use core::time::Duration;

fn main() {
    // Create and lock a mutex
    let counter = Mutex::new(0u32);
    // ...
}
```

## License

Proprietary. See [LICENSE](LICENSE) for details.

## Status

Phase 1 — Workspace and architecture foundation.
