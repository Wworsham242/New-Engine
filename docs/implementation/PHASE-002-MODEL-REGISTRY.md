# Phase 002 â€” World Registry, Model Metadata, and Solver Registration

## Purpose

Phase 002 converts the architecture metadata requirements from ADR-002,
ADR-003, and ADR-004 into executable validation.

The simulation now has two distinct registry concepts.

### World registry

Stable runtime identity for world dimensions such as:

- country;
- region;
- sector;
- energy type.

IDs are allocated canonically during world construction and become immutable.

### Model registry

Static model metadata for:

- variables;
- equations;
- solver groups;
- ownership;
- units;
- cadence;
- temporal semantics;
- provenance.

## Important limitation

The Phase-001 equations are still uncalibrated engineering scaffolds.

They are explicitly registered with `ORIGINAL_DESIGN` /
`PERFORMANCE_SIMPLIFICATION` provenance and descriptions stating that they are
not IFs production equations.

IFs remains an observed architecture / behavioral reference. Future model
equations should be independently derived from public literature, datasets, or
original design and registered with the correct provenance.

CMO remains a military reference model at the higher formation abstraction
defined by ADR-008.

## Validation now enforced

The registry rejects:

- duplicate variable IDs;
- duplicate variable keys;
- duplicate equation IDs;
- duplicate equation keys;
- duplicate solver-group IDs;
- missing input/output variables;
- unknown solver groups;
- unknown convergence variables;
- invalid solver policies;
- multiple ordinary writers for one authoritative variable.

## Developer CLI

```powershell
cargo run -p sim-tools -- model validate
cargo run -p sim-tools -- variables list
cargo run -p sim-tools -- equations list
cargo run -p sim-tools -- solvers list
```

## Next step

Phase 003 should replace the hard-coded global demo state with the first true
world registry + country-indexed dense state, beginning with multiple countries
and typed `Dense1`/`Dense2` storage rather than adding more placeholder model
equations.