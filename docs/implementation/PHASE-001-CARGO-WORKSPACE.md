# Phase 001 â€” Cargo Workspace and Deterministic Vertical Slice

## Purpose

Phase 001 turns the architecture documents into compiling Rust boundaries.

It deliberately implements only a tiny causal demonstration:

```text
primitive energy-capacity shock
 -> energy shortage / price response
 -> economy response
 -> governance revenue response
 -> authoritative commit
 -> BLAKE3 state hash
```

The equations in this phase are engineering placeholders.

They are **not** copied from International Futures (IFs), are **not** calibrated,
and are not intended to become the production equations merely because they
exist first.

IFs remains an architecture/behavior reference. CMO remains a military
operational-relationship reference at the higher abstraction defined in
ADR-008.

## Workspace

- `sim-types`
- `sim-storage`
- `sim-units`
- `sim-equations`
- `sim-contracts`
- `sim-state`
- `sim-kernel`
- `sim-random`
- `sim-tools`
- `subsystem-demographics`
- `subsystem-energy`
- `subsystem-economy`
- `subsystem-governance`

## Architecture demonstrated

- Rust 2024 Cargo workspace
- pinned stable toolchain channel
- typed subsystem boundaries
- single authoritative world commit per monthly tick
- primitive cause before downstream outcome
- Jacobi-style Economy/Energy contract snapshot
- deterministic damping and iteration cap
- subsystem ownership boundaries
- BLAKE3 canonical state hash
- baseline-vs-shock isolation
- headless CLI execution
- dense/sparse storage primitives without graphics dependency

## What is intentionally absent

- calibrated IFs-like equations
- agriculture
- infrastructure networks
- trade
- full demographic cohorts
- delayed pipelines
- persistence/replay files
- production PRNG implementation
- military formations/substeps
- UI

Those are added incrementally behind the architecture already committed.