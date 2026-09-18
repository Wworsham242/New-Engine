# ADR-001: Deterministic Discrete-Time Simulation Kernel in Rust

**Status:** Accepted  
**Date:** 2026-09-17  
**Decision owners:** Project architecture  
**Applies to:** New Engine simulation core  
**Related:** `../SYSTEM_ARCHITECTURE_V0_1.md`

## Context

New Engine requires a simulation kernel capable of coordinating tightly coupled economic, energy, agricultural, fiscal, demographic, infrastructure, environmental, international, and military systems without collapsing them into one global solver.

The kernel must support:

- deterministic execution and replay;
- explicit ownership of authoritative state;
- multiple subsystem cadences;
- fast military/operational substeps;
- slower structural updates;
- bounded local and cross-subsystem iteration;
- delayed effects and pipelines;
- baseline-vs-shock experiments;
- causal diagnostics;
- scalable parallelism without thread-order-dependent results.

The architecture also requires a clean separation between the simulation kernel and domain equations. The kernel schedules and coordinates calculations; it does not own economic, energy, military, demographic, or policy logic.

Rust is selected for the simulation kernel because its ownership model, enums, newtypes, algebraic data types, concurrency guarantees, zero-cost abstractions, and strong tooling fit the requirements for deterministic state ownership and explicit subsystem boundaries.

This ADR locks the first executable kernel decisions.

---

## Decision

### 1. Kernel implementation language

The authoritative simulation kernel will be implemented in **Rust**.

Target:

```text
Rust stable
Edition 2024
```

The workspace should pin the Rust toolchain used for authoritative builds through `rust-toolchain.toml` once implementation begins.

Rust is the default language for:

- simulation clock;
- scheduler;
- authoritative world state;
- subsystem traits;
- typed contracts;
- delayed-event queues;
- solver orchestration;
- deterministic random streams;
- snapshotting;
- replay;
- state hashing;
- diagnostics.

Other languages may be used for tooling, data preparation, visualization, or UI integration, but they do not own authoritative simulation state unless a later ADR explicitly changes this.

---

## 2. Time representation

Simulation time is discrete.

The kernel uses a monotonic integer tick:

```rust
#[derive(
    Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash
)]
pub struct Tick(pub u64);
```

No authoritative simulation equation may depend on wall-clock time.

Calendar interpretation is handled separately:

```rust
pub struct SimulationDate {
    pub year: i32,
    pub month: u8,
}
```

The kernel stores authoritative progression as `Tick`; calendar conversion belongs to the configured simulation profile.

---

## 3. Modern-world master cadence

For the initial modern-world simulation profile:

> **One master tick equals one calendar month.**

This is the synchronization cadence for authoritative state.

Reasons:

- energy, trade, inventories, readiness, sanctions, and shortages can change materially within a quarter;
- monthly state gives war and crisis systems enough temporal resolution without forcing the entire world model into daily updates;
- quarterly and annual systems can still execute only when due;
- operational military systems can substep internally.

The master cadence is profile-configurable at world creation but fixed for the duration of a run.

Changing cadence mid-run is not supported in v0.1.

---

## 4. Subsystem cadence

Each subsystem or scheduled model task declares a cadence.

Initial cadence classes:

```rust
pub enum Cadence {
    EveryMasterTick,
    EveryNMasterTicks(u32),
    Quarterly,
    Annual,
    InternalSubsteps(SubstepCadence),
}
```

Examples for the monthly modern profile:

```text
Military operations        weekly/daily internal substeps
Energy balancing           monthly
Trade/inventory adjustment monthly
Government cash flow       monthly
Macro solve                quarterly, with monthly accumulators
Investment                 quarterly
Demographic cohort aging   annual
Education attainment       annual
Long-run productivity      annual
Major capital pipelines    monthly progress / completion events
```

Subsystems may accumulate flows monthly while performing structural recalculation less frequently.

---

## 5. Master tick phases

Every master tick executes the following fixed phase sequence:

```text
0. BeginTick
1. ApplyExogenousInputs
2. ActivateDueDelayedEvents
3. PrepareInheritedState
4. FastPhysicalOperationalPass
5. CoreCoupledSolve
6. SocialStructuralPass
7. StrategicMilitaryPass
8. EnvironmentResourcePass
9. ReconciliationConstraintPass
10. QueueFutureEffects
11. CommitAuthoritativeState
12. HashAndRecordReplay
13. EmitDiagnostics
14. EndTick
```

The order is authoritative.

A future change to phase semantics requires an ADR and replay-version change.

---

## 6. State model

The kernel distinguishes four state views.

### 6.1 Committed state

`CommittedWorldState` is the authoritative state at the end of the previous tick.

During a tick it is immutable.

```rust
pub struct CommittedWorldState {
    // authoritative domain stores
}
```

### 6.2 Working state

Subsystems calculate into owned working state.

Working state is not globally mutable.

```rust
pub struct WorkingState<S> {
    pub subsystem: S,
}
```

### 6.3 Contract snapshot

Cross-subsystem data is exchanged through immutable typed contract snapshots.

```rust
pub struct ContractSnapshot {
    // versioned typed subsystem interfaces
}
```

### 6.4 Pending state

At the end of the tick, reconciled subsystem outputs become `PendingCommit`.

Only the commit phase can replace authoritative state.

This creates a clear boundary:

```text
Committed(t)
    -> calculations
    -> Pending(t+1)
    -> atomic logical commit
    -> Committed(t+1)
```

---

## 7. State ownership

Each authoritative variable has exactly one owning subsystem.

The kernel enforces the architectural rule:

> A subsystem may read external information only through declared contracts or kernel services.

Direct arbitrary mutation of another subsystem's state is prohibited.

Example:

```rust
pub trait Subsystem {
    type State;
    type Inputs;
    type Outputs;

    fn solve(
        &self,
        state: &Self::State,
        inputs: &Self::Inputs,
        ctx: &SolveContext,
    ) -> SolveResult<Self::State, Self::Outputs>;
}
```

Exact trait shapes may change during implementation, but ownership semantics do not.

---

## 8. Cross-subsystem contracts

Contracts are typed Rust structures.

Example:

```rust
pub struct EnergyToEconomy {
    pub production: EnergyQuantity,
    pub demand: EnergyQuantity,
    pub imports: EnergyQuantity,
    pub exports: EnergyQuantity,
    pub stocks: EnergyQuantity,
    pub price_index: EnergyPriceIndex,
    pub shortage: ShortageIndex,
    pub utilization: CapacityUtilization,
}

pub struct EconomyToEnergy {
    pub real_gdp: RealGdp,
    pub gdp_per_capita: GdpPerCapita,
    pub industrial_activity: SectorDemand,
    pub investment: InvestmentDemand,
    pub import_capacity: ImportCapacity,
    pub exchange_rate: ExchangeRateState,
}
```

Contracts are:

- immutable after publication for a solve iteration;
- versioned where save/replay compatibility requires it;
- explicit about units;
- explicit about temporal semantics;
- hashable or canonically serializable for diagnostics.

---

## 9. Core coupled iteration strategy

The initial core coupled group is:

```text
Economy
Energy
Agriculture
Governance/Fiscal
Infrastructure (selected fast/capacity interfaces)
```

### 9.1 Iteration semantics

Cross-subsystem outer iteration uses **Jacobi-style snapshots** in v0.1.

At iteration `n`:

1. all participating subsystems read the same immutable contract snapshot `C[n]`;
2. each subsystem solves independently against `C[n]`;
3. outputs are collected;
4. a deterministic barrier constructs `C[n+1]`;
5. convergence is measured;
6. either stop or repeat.

Conceptually:

```text
C[n]
 |----> Economy --------|
 |----> Energy ---------|
 |----> Agriculture ----|--> deterministic barrier --> C[n+1]
 |----> Governance -----|
 |----> Infrastructure -|
```

This is chosen over cross-subsystem Gauss-Seidel for v0.1 because it:

- makes iteration semantics obvious;
- prevents accidental order dependence;
- supports safe parallelism;
- improves reproducibility;
- makes baseline-vs-shock debugging easier.

Individual subsystem solvers may use their own deterministic internal iteration scheme.

---

## 10. Solver convergence

Every iterative group defines a `SolverPolicy`.

```rust
pub struct SolverPolicy {
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
    pub damping: f64,
    pub max_iterations: u32,
    pub non_convergence: NonConvergencePolicy,
}
```

Convergence is measured only on explicitly registered convergence variables.

Example:

```text
Energy:
    price index
    shortage index
    utilization
    demand

Economy:
    real GDP
    price level
    investment
    employment

Governance:
    revenue
    spending
    fiscal balance
```

A variable residual is:

```text
abs(new - old) <= absolute_tolerance
OR
abs(new - old) / max(abs(old), epsilon) <= relative_tolerance
```

The group converges only when all required residuals satisfy policy.

---

## 11. Damping

Damping is explicit, deterministic, and policy-controlled.

```text
published =
    old + damping * (new - old)
```

`damping = 1.0` means no damping.

Damping values are configuration data and are included in replay metadata.

No solver may silently invent an adaptive damping value in authoritative mode unless that algorithm itself is deterministic and versioned.

---

## 12. Non-convergence policy

The initial production policy is:

```rust
pub enum NonConvergencePolicy {
    CommitLastIterationWithDiagnostic,
    AbortStrictRun,
}
```

Normal gameplay:

```text
CommitLastIterationWithDiagnostic
```

Validation/research mode may use:

```text
AbortStrictRun
```

Non-convergence produces a structured diagnostic containing:

- tick;
- solver group;
- iteration count;
- largest residuals;
- affected variables;
- solver policy;
- damping value.

---

## 13. Local subsystem solvers

A subsystem may internally use:

- direct equations;
- accounting reconciliation;
- fixed-point iteration;
- deterministic Gauss-Seidel;
- deterministic Newton-like methods where appropriate;
- allocation solvers;
- market adjustment algorithms.

A subsystem's internal algorithm is its responsibility, provided that it:

- is deterministic;
- obeys input contracts;
- respects iteration limits;
- reports diagnostics;
- does not mutate foreign authoritative state.

---

## 14. Delayed events and pipelines

Delayed consequences use an ordered queue.

```rust
pub struct DelayedEvent {
    pub due_tick: Tick,
    pub subsystem: SubsystemId,
    pub entity: EntityId,
    pub sequence: u64,
    pub payload: DelayedEventPayload,
}
```

Canonical ordering:

```text
due_tick
subsystem_id
entity_id
sequence
```

This guarantees deterministic activation when many events are due simultaneously.

Examples:

- factory completion;
- ship delivery;
- training completion;
- repair completion;
- debt maturity;
- harvest;
- delayed health effect;
- demographic transition;
- reconstruction.

Delayed events represent causal processes, not generic arbitrary delays.

---

## 15. Internal military substeps

Military operations may require higher temporal resolution than the monthly world tick.

Military substeps operate inside the current master tick.

They may update military working state at daily or weekly resolution but cannot commit global authoritative civilian state independently.

Example:

```text
Master tick: September 2026

Military substeps:
    week 1
    week 2
    week 3
    week 4
    remainder/calendar reconciliation

Published at synchronization boundary:
    casualties
    equipment losses
    fuel consumption
    ammunition consumption
    infrastructure damage
    transport demand
    territorial/security changes
```

Major physical events that must affect other fast subsystems within the month may publish through explicitly defined intra-tick synchronization points in a future ADR.

v0.1 does not permit arbitrary cross-subsystem callbacks during substeps.

---

## 16. Randomness

Deterministic randomness is permitted, but uncontrolled randomness is not.

Requirements:

- one versioned PRNG algorithm;
- recorded root seed;
- scoped/keyed streams;
- no dependency on thread scheduling;
- no OS randomness during authoritative simulation after initialization.

Conceptual key:

```text
root_seed
+ subsystem_id
+ tick
+ entity_id
+ event_kind
+ draw_index
```

The implementation should prefer independently derivable streams or counter/key-based draws rather than one global mutable RNG.

The exact PRNG crate/algorithm will be selected in a dedicated implementation decision and pinned by dependency version.

---

## 17. Floating-point policy

v0.1 uses `f64` for continuous authoritative calculations unless a domain requires an exact integer/fixed-point quantity.

Examples that should remain integers/newtypes where possible:

- population counts;
- equipment counts;
- event sequence numbers;
- discrete capacity units;
- ticks.

Initial reproducibility target:

> deterministic for the same authoritative build, target architecture, data, configuration, and inputs.

v0.1 does not yet promise bit-identical floating-point results across all CPU architectures and compilers.

Authoritative builds must:

- disable unsafe fast-math assumptions;
- avoid iteration over unordered containers when order affects arithmetic;
- use stable reduction order;
- pin algorithm versions;
- test state hashes.

Cross-platform bit determinism is deferred to a later ADR.

---

## 18. Deterministic collections and ordering

Simulation correctness must never depend on randomized hash-map iteration order.

Rules:

- use indexed arrays/slices for hot authoritative state where practical;
- use stable IDs;
- use `BTreeMap`/sorted vectors when ordered map traversal is required;
- if `HashMap` is used, never depend on iteration order for authoritative calculations;
- sort keys explicitly before deterministic reductions.

---

## 19. Parallel execution

The scheduler may parallelize jobs that are independent inside the dependency graph.

v0.1 parallelism rule:

> Parallel execution may change wall-clock duration, never authoritative result.

Permitted pattern:

```text
immutable input snapshot
    -> independent indexed jobs
    -> separate output slots
    -> deterministic ordered reduction/barrier
```

Forbidden:

```text
parallel workers
    -> shared floating accumulator with timing-dependent update order
```

The engine should retain a single-thread deterministic reference mode for debugging and CI.

A parallel implementation must match the reference mode's state hash within the defined floating-point reproducibility target.

---

## 20. Scheduler representation

The scheduler operates from explicit task metadata.

Conceptual Rust types:

```rust
pub struct TaskId(pub u32);
pub struct SubsystemId(pub u16);

pub struct ScheduledTask {
    pub id: TaskId,
    pub subsystem: SubsystemId,
    pub phase: TickPhase,
    pub cadence: Cadence,
    pub dependencies: Vec<TaskId>,
}
```

Startup validation must reject:

- missing dependencies;
- illegal phase dependencies;
- ownership conflicts;
- duplicate authoritative writers;
- unresolved contract requirements.

Cycles are allowed only inside explicitly declared solver groups.

---

## 21. Tick phase type

```rust
#[repr(u8)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum TickPhase {
    BeginTick = 0,
    ApplyExogenousInputs = 1,
    ActivateDueDelayedEvents = 2,
    PrepareInheritedState = 3,
    FastPhysicalOperationalPass = 4,
    CoreCoupledSolve = 5,
    SocialStructuralPass = 6,
    StrategicMilitaryPass = 7,
    EnvironmentResourcePass = 8,
    ReconciliationConstraintPass = 9,
    QueueFutureEffects = 10,
    CommitAuthoritativeState = 11,
    HashAndRecordReplay = 12,
    EmitDiagnostics = 13,
    EndTick = 14,
}
```

Changing discriminants after save/replay formats depend on them requires explicit migration/version handling.

---

## 22. Commit semantics

A master tick has one global authoritative commit point.

Before commit:

- calculations are provisional;
- constraints may reconcile state;
- diagnostics may inspect differences;
- no external consumer should treat pending state as authoritative.

At commit:

```text
Pending(t+1) -> Committed(t+1)
```

The commit is logically atomic.

Implementation may use separate subsystem stores rather than one monolithic structure, but the simulation semantics are a single world-state transition.

---

## 23. State hashing

At every committed tick, the engine can compute an authoritative state hash.

Recommended initial algorithm:

```text
BLAKE3
```

Hash input must use canonical serialization/order, not raw Rust struct memory.

Do not hash:

- pointer addresses;
- padding bytes;
- wall-clock timestamps;
- diagnostic-only data;
- nondeterministically ordered collections.

State hashes support:

- determinism tests;
- replay validation;
- multiplayer debugging;
- regression detection;
- baseline/shock experiment verification.

---

## 24. Replay record

Each run records at minimum:

```rust
pub struct ReplayHeader {
    pub engine_version: String,
    pub architecture_version: String,
    pub data_version: String,
    pub simulation_profile: String,
    pub root_seed: [u8; 32],
    pub initial_state_hash: [u8; 32],
}
```

Per tick, record:

- player/AI authoritative actions;
- exogenous scenario inputs;
- externally injected events;
- solver-policy version;
- final state hash;
- notable solver failures/non-convergence.

Derived state does not need to be fully duplicated in the replay log if it can be deterministically regenerated.

Periodic snapshots may be stored for seek performance.

---

## 25. Save compatibility

Save-state schema and replay schema are versioned independently from Rust type layout.

Rust's in-memory representation is not a persistent file format.

Any serialization technology chosen later must support:

- explicit format version;
- migration or rejection policy;
- canonical ordering where hashing requires it;
- bounds checking;
- corrupt-input rejection.

---

## 26. Diagnostics

The kernel provides structured diagnostic hooks.

Required categories:

```rust
pub enum DiagnosticKind {
    SolverNonConvergence,
    ConstraintCorrection,
    ContractChange,
    DelayedEventActivated,
    StateHash,
    CausalTrace,
    Performance,
}
```

Diagnostics are not allowed to change authoritative results.

Diagnostic collection may be disabled or sampled for performance.

---

## 27. Causal tracing

Equation/subsystem execution should optionally emit causal edges:

```text
output variable
<- input variable
<- prior state
<- parameter
<- event/shock
```

A trace record should carry:

- tick;
- subsystem;
- equation/model ID;
- output ID;
- relevant input IDs;
- optional magnitude contribution where calculable;
- provenance.

Causal tracing is a developer and player-facing requirement, not an afterthought.

---

## 28. Baseline-vs-shock experiment support

The kernel must make controlled experiments easy.

Required developer workflow:

```rust
let baseline = run(initial.clone(), baseline_scenario);
let shock = run(initial, shock_scenario);

let delta = compare(baseline, shock);
```

The experiment harness must verify:

- identical initial state hash;
- identical configuration except declared intervention;
- deterministic replay;
- variable-level and subsystem-level deltas.

This is a first-class architectural feature because causal experiments are central to validating the model.

---

## 29. Error handling

Core simulation APIs use typed errors.

Panics are reserved for violated programmer invariants that cannot be sensibly recovered from.

Expected runtime conditions use `Result`.

Examples:

- invalid scenario input;
- corrupt save;
- unsatisfied contract;
- missing reference data;
- strict-mode non-convergence.

No subsystem may silently swallow an authoritative-state error.

---

## 30. Performance direction

The architecture favors:

- structure-of-arrays or compact arrays for hot numeric state;
- stable integer IDs;
- contiguous iteration;
- batched equations;
- sparse work scheduling;
- cadence gating;
- subsystem-local caches derived only from authoritative inputs;
- parallel immutable reads with deterministic output slots.

Avoid:

- string lookups in hot loops;
- pervasive heap allocation per entity per tick;
- trait-object dispatch in inner numeric loops;
- shared mutex-protected accumulators in authoritative calculations.

Rust abstractions should be used at system boundaries without sacrificing data-oriented hot paths.

---

## 31. ECS relationship

The simulation kernel is not required to be an ECS.

ECS may be appropriate for:

- sparse entities;
- military formations;
- infrastructure objects;
- events;
- map objects.

Dense system-of-systems numeric state may be better represented in dedicated subsystem arrays.

The scheduler and contract system remain authoritative regardless of storage strategy.

A separate ADR should define ECS scope if needed.

---

## 32. Initial crate/workspace direction

Recommended workspace shape:

```text
crates/
  sim-kernel/
  sim-types/
  sim-contracts/
  sim-state/
  sim-diagnostics/
  sim-experiments/
  subsystem-demographics/
  subsystem-economy/
  subsystem-energy/
  subsystem-governance/
```

Later:

```text
  subsystem-agriculture/
  subsystem-infrastructure/
  subsystem-education/
  subsystem-health/
  subsystem-environment/
  subsystem-human-development/
  subsystem-international/
  subsystem-military/
```

The exact workspace layout may adapt to the existing repository structure, but the dependency direction should remain:

```text
domain subsystem
    -> shared types/contracts
    -> kernel interfaces

kernel
    must not depend on domain implementation crates
```

---

## 33. Minimal kernel vertical slice

ADR-001 is considered implemented when the repository can run a headless Rust executable that performs:

```text
initial state
 -> monthly tick
 -> delayed-event activation
 -> Energy solve
 -> Economy/Energy coupled iteration
 -> Governance update
 -> reconciliation
 -> commit
 -> state hash
 -> replay record
```

The demonstration scenario should include:

```text
Month N:
    primitive oil-production capacity shock

Following months:
    Energy output/shortage/price changes
    economic response
    fiscal response
    delayed recovery/investment effect
```

The test must not directly script GDP.

---

## 34. Required tests

### 34.1 Identical-run determinism

Run the same scenario twice:

```text
hash(run A, tick n) == hash(run B, tick n)
```

for every committed tick.

### 34.2 Single-thread vs parallel

When parallel mode exists, compare final authoritative state/hash against reference mode.

### 34.3 Event ordering

Insert multiple delayed events due on the same tick and verify canonical ordering.

### 34.4 Solver repeatability

Identical input snapshot must produce identical iteration count, residuals, and outputs.

### 34.5 Replay regeneration

Replay inputs from the initial snapshot and reproduce all recorded state hashes.

### 34.6 Baseline-vs-shock isolation

Verify the only input difference between controlled experiment branches is the declared shock.

---

## 35. Consequences

### Positive

- Rust ownership reinforces authoritative state ownership.
- Typed contracts make subsystem seams explicit.
- Monthly master cadence supports modern crises without daily world recalculation.
- Jacobi outer coupling reduces hidden execution-order dependence.
- Immutable iteration snapshots permit safe parallelism.
- One commit point simplifies replay and debugging.
- Ordered delayed events support causal lags.
- State hashes provide a strong determinism test.
- The model can scale through cadence gating and subsystem-specific resolution.

### Costs

- Typed boundaries require more up-front engineering than shared mutable state.
- Jacobi coupling may require more iterations than Gauss-Seidel.
- Strict deterministic ordering can limit some easy parallel optimizations.
- Monthly state increases storage/work compared with quarterly-only simulation.
- Replay/save versioning adds infrastructure work early.
- Rust migration requires replacing the C++-oriented examples in Architecture v0.1.

These costs are accepted because correctness, explainability, and reproducibility are core project requirements.

---

## 36. Rejected alternatives

### One global daily tick

Rejected because most civilian systems do not require daily updates and the cost would be substantial.

### One global annual or quarterly tick

Rejected for the modern profile because energy, logistics, military readiness, trade disruption, sanctions, and crisis effects need finer synchronization.

### Shared mutable world object

Rejected because it obscures ownership and makes deterministic parallel execution and causal debugging harder.

### Cross-subsystem Gauss-Seidel as the default

Deferred/rejected for v0.1 because it creates more execution-order coupling. It may be reconsidered for particular solver groups if profiling demonstrates a need.

### One giant global numerical solver

Rejected because subsystem equations have different semantics, cadences, and time constants.

### Fully asynchronous event simulation

Rejected as the primary world model because the project needs clear synchronized economic/social state and controlled experiments. Event queues are retained for delayed processes inside a discrete-time framework.

### C++ as the authoritative kernel language

Superseded by this ADR. Rust is the authoritative simulation-kernel language unless a later ADR reverses the decision.

---

## 37. Follow-up ADRs

Recommended next decisions:

```text
ADR-002  Authoritative State Ownership and Typed Contracts
ADR-003  Equation Registry, Units, and Provenance
ADR-004  Solver Groups and Convergence Policies
ADR-005  Delayed Effects, Pipelines, and Expectations
ADR-006  Deterministic Randomness
ADR-007  Save/Replay Format and State Hashing
ADR-008  Military Operational Substeps
ADR-009  ECS Scope vs Dense Subsystem Storage
```

---

## 38. Architecture version impact

This ADR changes one statement in `SYSTEM_ARCHITECTURE_V0_1.md`:

```text
Primary implementation language: C++
```

becomes:

```text
Primary implementation language: Rust
```

The illustrative C++ structural section should also be replaced with a Rust structural direction in the next architecture document update.

No other architectural invariant is changed.

---

## Decision summary

New Engine will use a **Rust deterministic discrete-time simulation kernel** with:

- a monthly modern-world master tick;
- subsystem-specific cadences;
- military internal substeps;
- immutable committed state during calculation;
- one logical commit point per master tick;
- typed cross-subsystem contracts;
- Jacobi-style outer coupling for core systems;
- deterministic local solvers;
- bounded convergence policies;
- ordered delayed-event queues;
- deterministic parallelism;
- versioned replay;
- canonical state hashes;
- first-class baseline-vs-shock experiments;
- optional causal tracing.

This decision is the implementation foundation for the New Engine world simulation.
