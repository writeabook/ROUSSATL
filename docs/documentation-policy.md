# Documentation Policy

## Purpose

This document defines the authority of each document in the OSAL
repository, when each must be updated, and the status terminology
used throughout.

## Source-of-truth hierarchy

When documents disagree, the higher-priority source wins:

1. **Rust public API / Cargo manifests** — what actually compiles.
   Code is the ultimate authority on current behavior.

2. **`docs/behavior-contract.md`** — normative backend conformance
   requirements. Describes what backends **MUST** do.

3. **`docs/adr/*.md`** — why specific design decisions were made.
   Accepted ADRs record the rationale at the time of decision.

4. **`docs/architecture.md`** — stable layer boundaries, dependency
   directions, and crate responsibility descriptions.

5. **`docs/*-foundation-slice.md`** — per-capability implementation
   status, component breakdown, and deferred items for a specific
   feature slice.

6. **`README.md`** — summary snapshot for new contributors. Must not
   introduce new semantic claims not present in higher-priority
   documents.

7. **`CHANGELOG.md`** — record of what changed when, grouped by
   development phase.

## Document responsibilities

| Document | Answers | Audience |
|----------|---------|----------|
| Code / rustdoc | "What does it do right now?" | Compiler, IDE, all developers |
| Behavior contract | "What must every backend do?" | Backend implementors, test authors |
| ADRs | "Why was this decision made?" | Maintainers, future contributors |
| Architecture | "How do the pieces fit together?" | New contributors, reviewers |
| Foundation slices | "What is done and what is deferred for X?" | Contributors to feature X |
| README | "What is this project and where is it?" | First-time visitors |
| CHANGELOG | "What changed between milestones?" | Upgrading users |
| Documentation policy | "How do I keep docs consistent?" | All contributors |

## When a document must be updated

### API signature change

- Code and rustdoc: always
- Behavior contract: if the change affects observable semantics
- CHANGELOG: always (under the current phase)

### Behavior change (same API, different outcome)

- Code and tests: always
- Behavior contract: always (normative behavior changed)
- ADR: if architecturally significant
- CHANGELOG: always

### Implementation-only change (refactor, perf, fix without API change)

- Code and tests: always
- CHANGELOG: when externally relevant (bug fix, performance)

### New architecture decision

- New ADR with sequential number
- ADR index in README updated
- Architecture document: when layer boundaries or dependencies change

### New milestone

- CHANGELOG: new phase section
- README: status section updated

### When a capability moves from Deferred to Implemented

- Behavior contract: remove Deferred markers, add normative
  requirements
- Architecture document: update crate maturity labels
- Foundation slice: update status
- README: update capability matrix

## Status terminology

Use these terms consistently across all documents:

| Term | Meaning |
|------|---------|
| `Validated` | API, implementation, and contract tests complete |
| `Implemented` | Implemented; contract or edge-case verification ongoing |
| `Foundation` | Foundation semantics complete; advanced features deferred |
| `Planned` | Design exists or sketched; implementation not started |
| `Deferred` | Explicitly deferred to a future phase with recorded rationale |
| `N/A` | Not applicable to this layer or backend |
| `Skeleton` | Crate or module exists as workspace placeholder only |

## ADR rules

- Accepted ADRs **MUST NOT** be silently rewritten. When a design
  changes, create a new ADR and mark the old one as **Superseded**
  with a link to the replacement.
- ADR numbers are sequential and never reused.
- Each ADR **MUST** record its status (`Proposed`, `Accepted`,
  `Superseded`, or `Deprecated`) and the date of status change.
- The ADR index in `README.md` **MUST** list every ADR in numeric
  order.

## Marking Deferred or Future work

- Use **Deferred** for items with a recorded decision to postpone
  (usually linked to an ADR or foundation slice).
- Use **Future** or **Planned** for items on the roadmap without a
  formal deferral decision.
- Behavior contract sections marked **Deferred** are not part of
  current backend conformance requirements.
- Architecture document must distinguish between current
  implementation and target extension using separate diagrams or
  clearly labelled sections.

## Review checklist

Before marking a document change complete, verify:

- [ ] No new semantic claims introduced in README that aren't in
  behavior-contract or architecture
- [ ] Feature flags and allocation model consistent across
  architecture.md and behavior-contract.md §2
- [ ] Unimplemented capabilities (EventFlags, ISR traits, BSP) not
  described as current
- [ ] ADR index in README includes all ADRs
- [ ] CHANGELOG phase numbering matches README status
- [ ] Status terminology matches the table above
