# ADR 0012: CI Validation Gates

## Status

Accepted (2026-07-07)

## Context

The CI workflow must enforce quality gates without blocking
development on checks that are not yet meaningful. The initial CI
setup (`ci.yml`) included parallel clippy jobs targeting individual
backend feature combinations. Some of those combinations do not
compile cleanly because `--all-targets` pulls in backend-specific
examples under the wrong feature.

## Decision

The following commands form the MVP CI validation gates:

```bash
cargo test --workspace --features testkit
cargo clippy --workspace --all-targets --features testkit -- -D warnings
cargo fmt --all -- --check
cargo check -p osal --no-default-features --features backend-posix
cargo check -p osal --no-default-features --features backend-mock
```

These are runnable locally and in CI without matrix explosion.

### What is explicitly NOT gated

```bash
# NOT gated — pulls examples/ under the wrong feature
cargo clippy -p osal --all-targets --no-default-features --features backend-posix
cargo clippy -p osal --all-targets --no-default-features --features backend-mock
```

These commands attempt to compile backend-specific facade examples
under a single feature, which causes unresolved-import errors for
types only available from the other backend. They are not useful
quality checks at this stage and are excluded from the gate.

## Rationale

- The workspace-level `--features testkit` selects both backends and
  exercises the full contract matrix.
- `cargo fmt --check` is deterministic and fast.
- `cargo check` with each backend flag confirms the facade compiles
  for both targets without pulling in examples.
- Keeping the gate small avoids CI churn while still catching
  regressions.

## Consequences

- CI configuration should align with these five commands.
- Backend-specific example compilation is exercised by
  `cargo check -p osal --features backend-{posix,mock} --examples`
  separately if needed.
- Additional gates (benchmarks, coverage, docs link check) can be
  added later without changing this baseline.
