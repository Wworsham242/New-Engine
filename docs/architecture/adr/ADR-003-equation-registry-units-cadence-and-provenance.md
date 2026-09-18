# ADR-003: Equation Registry, Units, Cadence, and Provenance

**Status:** Proposed  
**Date:** 2026-09-17  
**Decision owners:** Project architecture  
**Applies to:** New Engine simulation model layer  
**Related:** `ADR-001-deterministic-discrete-time-kernel.md`, `ADR-002-authoritative-state-ownership-and-typed-contracts.md`, `../SYSTEM_ARCHITECTURE_V0_1.md`

## Context

ADR-001 established a deterministic Rust simulation kernel with a monthly master tick, subsystem-specific cadences, bounded solver iteration, deterministic replay, and one logical commit point per tick.

ADR-002 established single-owner authoritative state, typed IDs, unit-safe newtypes, dense/sparse storage guidance, and immutable typed contracts for cross-subsystem communication.

The next architectural requirement is to define how model equations and transformations are represented.

Without an explicit equation registry, the project risks:

- equations being scattered across subsystem code without metadata;
- undocumented ownership;
- hidden lags;
- unclear cadence;
- unit mismatches;
- accidental cross-subsystem dependencies;
- difficult causal tracing;
- difficulty validating IFs-inspired architecture against original implementations;
- inability to inspect or visualize the model;
- scheduler rules being duplicated manually;
- equation changes that silently alter simulation semantics.

The engine must preserve native compiled Rust performance while also making its model graph inspectable.

This ADR defines:

- equation identity;
- equation metadata;
- units;
- equation classes;
- cadence;
- lag semantics;
- provenance;
- registration;
- validation;
- scheduler integration;
- causal graph generation;
- Cargo/workspace organization.

---

# Decision

## 1. Equations are first-class registered model components

Every significant authoritative transformation must have a stable equation/model identifier and metadata.

"Equation" includes more than literal algebra.

It includes:

- accounting identities;
- stock-flow updates;
- behavioral functions;
- market adjustment;
- allocation routines;
- transition models;
- empirical response functions;
- constraint rules;
- aggregation rules;
- iterative solve components.

The registry describes what the model does.

The actual calculation remains compiled Rust.

---

## 2. Stable Equation IDs

Each registered equation has a stable typed identifier.

```rust
#[derive(
    Copy,
    Clone,
    Debug,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
)]
pub struct EquationId(pub u32);
```

Equation IDs must be stable within a model-data version.

They must not depend on nondeterministic discovery order.

The registry also carries a stable human-readable key:

```rust
pub struct EquationKey(pub &'static str);
```

Example:

```text
economy.gdp.real
energy.market.price_adjustment
agriculture.crop.yield
demographics.population.transition
military.readiness.update
```

The string key is used for:

- diagnostics;
- data mapping;
- causal traces;
- development tools;
- migration.

The numeric ID is used in hot runtime paths.

---

## 3. Variable IDs are separate from Equation IDs

Equations and variables must not share one identifier namespace.

```rust
pub struct VariableId(pub u32);
pub struct EquationId(pub u32);
```

An equation may:

- produce one authoritative variable;
- produce several related variables;
- contribute to a solver group result.

A variable may be affected by more than one model component only when ownership and combination semantics are explicit.

---

## 4. Equation metadata

Each registered equation has metadata similar to:

```rust
pub struct EquationMetadata {
    pub id: EquationId,
    pub key: EquationKey,
    pub owner: SubsystemId,
    pub class: EquationClass,
    pub cadence: EquationCadence,
    pub inputs: &'static [EquationInput],
    pub outputs: &'static [EquationOutput],
    pub lag: LagSemantics,
    pub iterative: IterationSemantics,
    pub provenance: &'static [ProvenanceTag],
    pub description: &'static str,
}
```

Metadata is declarative.

It does not contain mutable runtime state.

---

## 5. Equation classes

The registry defines a closed initial enum:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum EquationClass {
    AccountingIdentity,
    StockFlow,
    Behavioral,
    Capacity,
    Allocation,
    MarketAdjustment,
    Empirical,
    Lag,
    Transition,
    Constraint,
    Aggregation,
    SolverComponent,
}
```

The class is descriptive and diagnostic.

It does not automatically choose an algorithm.

---

## 6. Inputs are explicitly declared

Each input describes:

- variable or contract field;
- temporal reference;
- required/optional status;
- expected unit;
- source ownership.

Conceptually:

```rust
pub struct EquationInput {
    pub source: InputSource,
    pub temporal: TemporalReference,
    pub unit: UnitId,
    pub requirement: InputRequirement,
}
```

---

## 7. Input sources

```rust
pub enum InputSource {
    LocalVariable(VariableId),
    ContractField(ContractFieldId),
    Parameter(ParameterId),
    ExogenousInput(ExogenousInputId),
    DelayedEvent(EventKindId),
}
```

This makes hidden dependencies harder to create.

---

## 8. Outputs are explicitly declared

```rust
pub struct EquationOutput {
    pub variable: VariableId,
    pub unit: UnitId,
    pub semantics: OutputSemantics,
}
```

Initial semantics:

```rust
pub enum OutputSemantics {
    Set,
    AddFlow,
    SubtractFlow,
    Accumulate,
    ConstraintBound,
    SolverContribution,
}
```

The owner of the output variable must match the equation's subsystem unless an explicitly approved cross-owner event/contract mechanism is used.

---

## 9. Temporal references are explicit

An input must declare which time state it reads.

```rust
pub enum TemporalReference {
    CurrentCommitted,
    CurrentIteration,
    PreviousTick(u32),
    RollingAverage { ticks: u32 },
    RollingSum { ticks: u32 },
    Trend { ticks: u32 },
    PipelineState,
}
```

This avoids ambiguous code such as:

```text
"uses GDP"
```

when the real model requires:

```text
previous-year GDP growth average
```

---

## 10. No implicit one-tick delay

Dependencies are not automatically delayed.

The model must explicitly define timing.

Examples:

```text
Energy price -> monthly industrial demand
    CurrentIteration

Capital stock -> productive capacity
    CurrentCommitted

Education attainment -> productivity
    RollingAverage or lagged structural state
```

This is important because observed IFs architecture demonstrated both same-wave and delayed responses.

---

## 11. Cadence is equation metadata

Each equation declares when it is eligible to run.

```rust
pub enum EquationCadence {
    EveryMasterTick,
    EveryNMasterTicks(u32),
    Quarterly,
    Annual,
    OnEvent(EventKindId),
    SolverGroup(SolverGroupId),
    InternalSubstep(SubstepCadenceId),
}
```

The scheduler uses this metadata.

Subsystem code must not contain scattered ad-hoc calendar checks when cadence can be declared centrally.

---

## 12. Calendar-aware cadence resolution

For the monthly modern profile:

```text
EveryMasterTick -> every month
Quarterly       -> configured quarter boundaries
Annual          -> configured annual boundary
```

Cadence resolution is handled by the kernel calendar service.

No equation should hard-code:

```rust
if tick % 12 == 0
```

for calendar semantics.

---

## 13. Solver-group membership

Iterative equations can belong to named solver groups.

```rust
pub struct SolverGroupId(pub u16);
```

Examples:

```text
core.macro_energy
energy.market
agriculture.market
economy.production_income
governance.fiscal
```

Metadata declares membership:

```rust
pub enum IterationSemantics {
    None,
    LocalGroup(SolverGroupId),
    OuterCoupledGroup(SolverGroupId),
}
```

---

## 14. Solver groups have their own metadata

```rust
pub struct SolverGroupMetadata {
    pub id: SolverGroupId,
    pub key: &'static str,
    pub owner: SolverGroupOwner,
    pub policy: SolverPolicyId,
    pub convergence_variables: &'static [VariableId],
}
```

The group is the convergence unit.

An equation does not individually decide when the outer world loop has converged.

---

## 15. Units are registered explicitly

The project maintains a unit registry.

Initial unit categories include:

```text
dimensionless
currency
currency per time
population
population per time
energy
power/capacity
mass
volume
area
distance
time
rate
fraction
index
price
price per physical quantity
```

---

## 16. Unit IDs

```rust
pub struct UnitId(pub u16);
```

Equation metadata references unit IDs.

Runtime hot loops continue to use Rust newtypes where practical.

The registry exists for:

- validation;
- data loading;
- tooling;
- documentation;
- diagnostics.

---

## 17. Unit metadata

```rust
pub struct UnitMetadata {
    pub id: UnitId,
    pub key: &'static str,
    pub dimension: DimensionSignature,
    pub scale: f64,
}
```

Example keys:

```text
people
usd_2025
usd_2025_per_month
mwh
mw
hectare
fraction_0_1
index_100
```

---

## 18. Units and currency vintages

Monetary units must not simply be called `USD`.

They must encode:

- real/nominal;
- price base;
- time basis if flow.

Examples:

```text
usd_nominal
usd_2025_real
usd_2025_real_per_month
```

This avoids mixing incompatible values.

---

## 19. Physical and monetary quantities stay separate

Example:

Energy:

```text
Oil production      -> physical quantity
Oil price           -> currency / physical quantity
Oil export value    -> currency / time
```

These must not share a generic scalar.

---

## 20. Rates are semantically distinguished

A `Rate` may represent:

- fraction;
- annualized growth;
- monthly growth;
- hazard;
- probability;
- tax rate.

Metadata must record semantics where ambiguity exists.

A future refinement may introduce more specialized Rust newtypes.

---

## 21. Variable metadata

Each authoritative or important derived variable is registered.

```rust
pub struct VariableMetadata {
    pub id: VariableId,
    pub key: &'static str,
    pub owner: SubsystemId,
    pub unit: UnitId,
    pub kind: VariableKind,
    pub temporal: VariableTemporalSemantics,
    pub scope: VariableScope,
    pub provenance: &'static [ProvenanceTag],
}
```

---

## 22. Variable kinds

```rust
pub enum VariableKind {
    Stock,
    Flow,
    Rate,
    Index,
    ParameterizedState,
    Derived,
}
```

---

## 23. Variable temporal semantics

```rust
pub enum VariableTemporalSemantics {
    BeginningOfPeriod,
    EndOfPeriod,
    PeriodTotal,
    PeriodAverage,
    InstantaneousAtSyncPoint,
}
```

This is required for correct equation composition.

---

## 24. Variable scope

```rust
pub enum VariableScope {
    Global,
    Country,
    Region,
    Province,
    GridCell,
    Sector,
    Commodity,
    Cohort,
    Facility,
    Formation,
    Composite,
}
```

Dimensions may be refined through separate dimension metadata.

---

## 25. Dimension metadata

Multidimensional variables require explicit dimensions.

Conceptually:

```rust
pub struct DimensionSignature {
    pub axes: &'static [DimensionId],
}
```

Examples:

```text
GDP:
    Country

EnergyProduction:
    Country x EnergyType

Population:
    Country x Age x Sex

CropProduction:
    Country x CropType
```

This reflects an important lesson from IFs: dimension-aware dependencies matter.

---

## 26. Equation dimensional validation

At startup/build validation:

- every input unit must match the declared expected dimension;
- every output unit must match the target variable;
- dimension signatures must be compatible;
- aggregation across dimensions must be explicit.

The system should detect mistakes such as:

```text
MWh + USD
```

or:

```text
country-level GDP assigned into sector-level output
```

---

## 27. Provenance is first-class metadata

Each equation can carry one or more provenance tags.

Initial enum:

```rust
pub enum ProvenanceTag {
    OriginalDesign,
    IfsObservedArchitecture,
    PublicLiterature,
    DatasetDerived,
    MilitaryReferenceModel,
    PerformanceSimplification,
    GameplayAbstraction,
}
```

---

## 28. Provenance does not imply copied implementation

`IfsObservedArchitecture` means:

> the architectural relationship or response pattern was informed by observation of IFs behavior or exposed metadata.

It does not mean:

- copied source;
- copied equation;
- copied parameter;
- runtime dependency.

---

## 29. Source references

Equations informed by public literature or datasets should optionally include references.

```rust
pub struct SourceReference {
    pub key: &'static str,
    pub citation: &'static str,
}
```

The runtime does not need full citation text in hot memory.

References may be compiled into development metadata or generated documentation.

---

## 30. IFs refer-back field

For equations or subsystem seams inspired by IFs analysis, metadata may optionally include:

```rust
pub struct ReferenceObservation {
    pub source_model: &'static str,
    pub observation_key: &'static str,
}
```

Example:

```text
source_model: "IFs"
observation_key: "energy_to_economy_bridge"
```

Detailed notes live in documentation, not code comments copied from IFs.

---

## 31. Registration is compile-time/declarative where practical

The registry should not depend on runtime filesystem discovery.

Preferred:

- Rust static descriptors;
- explicit crate-level registries;
- optional proc-macro assistance.

Avoid:

- dynamic plugin scanning for core authoritative equations;
- string-based runtime equation lookup in hot loops.

---

## 32. Cargo workspace integration

Each subsystem crate registers its equation metadata locally.

Example workspace:

```text
crates/
  sim-types/
  sim-units/
  sim-equations/
  sim-contracts/
  sim-kernel/

  subsystem-economy/
  subsystem-energy/
  subsystem-agriculture/
  subsystem-governance/
  subsystem-demographics/
  ...
```

`sim-equations` owns shared registry types.

Subsystem crates own their equation descriptors.

---

## 33. No direct subsystem crate dependency

As established by ADR-002:

```text
subsystem-economy
    X-> subsystem-energy
```

Instead:

```text
subsystem-economy
    -> sim-equations
    -> sim-contracts
```

The kernel composes registries.

---

## 34. Initial registration mechanism

v0.1 uses explicit static slices.

Example:

```rust
pub static ECONOMY_EQUATIONS: &[EquationMetadata] = &[
    EQUATION_GDP_REAL,
    EQUATION_HOUSEHOLD_CONSUMPTION,
    EQUATION_INVESTMENT,
];
```

Subsystem crate exports:

```rust
pub fn equation_registry() -> &'static [EquationMetadata] {
    ECONOMY_EQUATIONS
}
```

This is intentionally simple and auditable.

---

## 35. Proc macros are optional, not required in v0.1

Cargo availability makes procedural macros practical, but ADR-003 does not require them initially.

Potential future form:

```rust
#[equation(
    key = "energy.market.price_adjustment",
    owner = "energy",
    cadence = "monthly",
    class = "market_adjustment"
)]
fn update_energy_price(...) -> ...
```

This may reduce metadata boilerplate later.

Initial implementation should first prove the registry design without macro complexity.

---

## 36. If proc macros are added, they must generate metadata only

A proc macro may:

- create IDs/keys;
- build descriptors;
- validate declared units;
- add registry entries.

It should not hide complex model logic.

Equation code must remain readable as ordinary Rust.

---

## 37. Registry assembly

The kernel builds a global immutable registry at startup.

Conceptually:

```rust
pub struct EquationRegistry {
    equations: Vec<&'static EquationMetadata>,
    variables: Vec<&'static VariableMetadata>,
    solver_groups: Vec<&'static SolverGroupMetadata>,
}
```

Assembly order is deterministic.

---

## 38. Registry validation

Startup validation must reject:

- duplicate equation IDs;
- duplicate equation keys;
- duplicate variable IDs;
- duplicate variable keys;
- missing inputs;
- invalid output ownership;
- unit mismatches;
- unknown contract fields;
- invalid cadence;
- illegal temporal dependency;
- unregistered solver groups;
- ambiguous multiple writers;
- cycles outside declared solver groups.

---

## 39. One writer rule

By default:

> one authoritative variable has one equation-family writer.

Exceptions:

- additive flows;
- solver contributions;
- event accumulation;
- explicitly declared reconciliation.

These combination semantics must be registered.

---

## 40. Additive flow registry

Example:

Population deaths may receive flows from:

```text
baseline mortality
war casualties
disaster casualties
epidemic mortality
```

Demographics still owns authoritative population state.

Flow contributors register as additive inputs to a named accumulation target.

Conceptually:

```rust
pub struct FlowAccumulatorMetadata {
    pub target: VariableId,
    pub contributors: &'static [EquationId],
}
```

---

## 41. Scheduler uses registry metadata

The scheduler asks:

```text
Which equations are due this tick?
Which solver group owns them?
Which dependencies are required?
Which contracts must exist?
```

It does not hard-code every equation name.

---

## 42. Runtime execution remains subsystem-owned

The registry does not imply a generic interpreter.

Preferred:

```rust
economy.solve_quarter(...)
```

rather than:

```rust
for equation in registry:
    interpret(equation.expression)
```

Metadata and execution are linked through subsystem-owned dispatch.

---

## 43. Dispatch strategies

Allowed approaches:

- static function tables;
- enum dispatch;
- generated dispatch;
- subsystem-level solve functions with equation tracing internally.

The exact mechanism may vary by subsystem.

The invariant is that registered metadata corresponds to actual executable model logic.

---

## 44. Equation tracing

When diagnostics are enabled, execution can emit:

```rust
pub struct EquationTrace {
    pub tick: Tick,
    pub equation: EquationId,
    pub entity: EntityTraceScope,
    pub outputs: Vec<TraceValue>,
}
```

Input contribution detail may be included where practical.

---

## 45. Causal graph generation

The registry can generate a declared dependency graph:

```text
Input Variable / Contract Field
    -> Equation
    -> Output Variable
```

This graph supports:

- architecture visualization;
- cycle detection;
- dependency audits;
- IFs comparison;
- causal explanation;
- test coverage analysis.

---

## 46. Declared graph is not execution order

Important distinction:

```text
dependency graph != scheduler execution order
```

Feedback cycles may exist.

The scheduler uses:

- cadence;
- phase;
- solver-group metadata;
- barriers;
- iteration policy.

This directly incorporates a lesson from the IFs audit: dependency metadata must not be mistaken for execution schedule.

---

## 47. Lag semantics are model metadata

The registry should distinguish:

```text
same synchronization point
previous committed tick
moving average
explicit delayed pipeline
cohort transition
construction completion
```

This makes delayed second-order effects inspectable.

---

## 48. Pipeline-generating equations

An equation may create future pipeline entries.

Metadata should identify this:

```rust
pub enum SideEffectClass {
    None,
    EmitsDelayedEvent(EventKindId),
    CreatesPipeline(PipelineTypeId),
}
```

Pipeline/event generation still occurs through kernel-approved interfaces.

---

## 49. Equation purity

Where practical, equation functions should resemble:

```rust
fn equation(
    inputs: &Inputs,
    params: &Parameters,
) -> Output
```

State mutation belongs in controlled application phases.

This enables:

- unit tests;
- differential tests;
- property tests;
- benchmarking.

---

## 50. Stateful solvers are permitted

Not all model logic can be pure.

A market solver may require iterative temporary state.

That temporary state is:

- local working state;
- not authoritative until commit;
- not hidden static/global memory.

---

## 51. Parameter registry

Parameters receive stable IDs too.

```rust
pub struct ParameterId(pub u32);
```

Metadata:

```rust
pub struct ParameterMetadata {
    pub id: ParameterId,
    pub key: &'static str,
    pub unit: UnitId,
    pub owner: SubsystemId,
    pub provenance: &'static [ProvenanceTag],
}
```

---

## 52. Parameters are externally inspectable

Developer tools should be able to ask:

```text
Which parameters affect this equation?
What are their units?
Where did they come from?
```

This is important for calibration and scenario design.

---

## 53. Scenario modifiers target parameters/state, not equation code

Scenario data may alter:

- parameters;
- exogenous inputs;
- initial state;
- physical capacity;
- policy state.

It should not dynamically replace arbitrary Rust function bodies in v0.1.

---

## 54. Equation versioning

Equation semantics may evolve.

Each equation carries a model version or semantic version field.

Conceptually:

```rust
pub struct ModelVersion(pub u16);
```

This supports:

- replay diagnostics;
- save migration;
- experiment reproducibility.

---

## 55. Model-set version

The full equation registry produces a canonical model-set hash.

Hash inputs include:

- equation IDs;
- equation keys;
- semantic versions;
- variable registry;
- parameter registry;
- contract schema versions;
- solver policies.

The replay header records this model-set hash.

---

## 56. Equation hash does not hash machine code

The registry hash describes semantic metadata/version identity.

It is not a cryptographic hash of compiled instructions.

Build version is recorded separately.

---

## 57. Equation documentation generation

Cargo tooling should be able to generate model documentation from registry metadata.

Example output:

```text
Equation:
    energy.market.price_adjustment

Owner:
    Energy

Class:
    MarketAdjustment

Cadence:
    Monthly

Inputs:
    supply
    demand
    stocks
    imports
    price[t-1]

Outputs:
    energy_price_index

Solver Group:
    energy.market

Provenance:
    OriginalDesign
    PublicLiterature
```

---

## 58. Registry CLI tooling

The workspace should eventually provide a command such as:

```powershell
cargo run -p sim-tools -- equations list
cargo run -p sim-tools -- equations show energy.market.price_adjustment
cargo run -p sim-tools -- graph export
```

This is not required for first implementation but is an intended architecture capability.

---

## 59. Unit validation CLI

Potential tool:

```powershell
cargo run -p sim-tools -- model validate
```

Checks:

- registry integrity;
- units;
- dimensions;
- ownership;
- cadence;
- solver cycles;
- contract references.

---

## 60. Cargo feature policy

Core equation semantics should not disappear unpredictably through Cargo features.

Features may control:

- diagnostics;
- tracing;
- heavy research models;
- optional visualization;
- benchmark tooling.

Features should not silently alter the authoritative model without changing the recorded simulation profile/model-set hash.

---

## 61. Debug vs release behavior

Validation should run in all authoritative modes where practical.

Heavy development-only checks may be behind:

```rust
debug_assert!
```

or diagnostic features.

Release builds must not skip checks that could permit silent state corruption.

---

## 62. Numeric precision metadata

Variables may declare preferred numeric representation.

Examples:

```text
Population -> u64
EquipmentCount -> u32/u64
Rates -> f64
Currency aggregates -> f64 initially
```

The registry can record numeric class.

Exact fixed-point decisions are deferred.

---

## 63. Equation error policy

Equation execution should not silently produce invalid numeric output.

Checks may include:

- finite numbers;
- expected bounds;
- nonnegative stocks;
- probability range;
- capacity constraints.

Violation produces:

- diagnostic;
- deterministic correction if policy allows;
- strict-run abort where configured.

---

## 64. NaN policy

Authoritative state must not contain NaN.

A NaN entering pending authoritative state is an error.

Infinity is also rejected unless a specific domain explicitly defines it, which v0.1 does not.

---

## 65. Bounds metadata

Variables may declare bounds:

```rust
pub enum Bounds {
    Unbounded,
    NonNegative,
    UnitInterval,
    Range { min: f64, max: f64 },
}
```

Reconciliation can use these declarations.

---

## 66. Equation preconditions

An equation may declare preconditions.

Examples:

```text
population > 0
capacity >= 0
denominator != 0
```

Preconditions support diagnostics and testing.

---

## 67. Equation invariants

Subsystems may declare postconditions/invariants.

Examples:

```text
energy stocks >= 0
shares sum approximately to 1
population accounting balances
government budget identity balances
```

---

## 68. Calibration metadata

Empirical equations should identify calibration context.

Conceptually:

```rust
pub struct CalibrationMetadata {
    pub dataset_key: &'static str,
    pub calibration_period: &'static str,
    pub geographic_scope: &'static str,
}
```

This is especially important when a relationship may not generalize across eras.

---

## 69. Era/profile applicability

An equation may declare applicability.

```rust
pub enum Applicability {
    AllProfiles,
    Profiles(&'static [SimulationProfileId]),
    EraRange(EraRangeId),
}
```

This supports scalable world complexity.

---

## 70. Alternate model implementations

A variable/equation family may have alternate implementations for different fidelity profiles.

Example:

```text
Trade:
    pooled market model
    bilateral network model
```

Only one implementation is active for a given model slot unless explicitly combined.

The registry records which implementation is active.

---

## 71. Model slot abstraction

Conceptually:

```rust
pub struct ModelSlotId(pub u16);
```

Example:

```text
trade.balance_model
energy.price_model
migration.behavior_model
```

Simulation profile chooses implementation.

---

## 72. Alternate implementations keep contract meaning stable

A high-fidelity model and simplified model should publish compatible contract semantics where possible.

This allows performance scaling without rewriting downstream systems.

---

## 73. Dependency visibility

Developer tools must be able to answer:

```text
What directly affects X?
What does X directly affect?
Which subsystem owns the relationship?
What cadence does it use?
Is it same-tick or lagged?
```

This is a core model-inspection requirement.

---

## 74. IFs comparison workflow

The registry enables controlled architectural comparison with IFs observations.

Example:

```text
IFs observation:
    Energy production connects strongly to Economy.

New Engine graph:
    EnergyProduction
      -> EnergyToEconomy
      -> industrial cost / supply / trade equations
```

The goal is to compare causal architecture, not reproduce equations.

---

## 75. Initial equation registry scope

The first vertical slice should register only enough equations to exercise the architecture.

Minimum:

### Demographics

```text
population carry-forward
working-age population
```

### Energy

```text
production
capacity constraint
demand
shortage
price adjustment
stocks
```

### Economy

```text
industrial output response
household consumption
investment
GDP accounting
```

### Governance

```text
tax revenue
government spending
fiscal balance
```

---

## 76. Initial controlled experiment

The first registered shock experiment:

```text
oil production capacity reduction
```

Expected causal route:

```text
capacity shock
 -> energy production
 -> shortage / stocks
 -> energy price
 -> industrial output
 -> GDP / income
 -> tax revenue / fiscal balance
 -> investment response
 -> future energy capacity
```

No direct GDP shock equation is allowed.

---

## 77. Initial Cargo workspace implementation

Recommended first crates:

```text
crates/
  sim-types/
  sim-units/
  sim-equations/
  sim-contracts/
  sim-state/
  sim-kernel/
  subsystem-demographics/
  subsystem-energy/
  subsystem-economy/
  subsystem-governance/
  sim-tools/
```

---

## 78. `sim-equations` responsibilities

`sim-equations` owns:

- EquationId;
- VariableId;
- ParameterId;
- UnitId references;
- metadata structs;
- equation-class enums;
- temporal-reference enums;
- registry validation;
- solver-group metadata;
- provenance metadata.

It does not own domain equations.

---

## 79. `sim-units` responsibilities

`sim-units` owns:

- unit registry;
- dimensional signatures;
- conversion metadata;
- common unit-safe newtypes or traits;
- validation helpers.

It should remain lightweight.

---

## 80. `sim-tools` responsibilities

Developer-facing Cargo binary:

```text
model validate
model list-equations
model show-equation
model export-graph
model list-variables
model list-parameters
```

Later:

```text
experiment compare
trace why
```

---

## 81. Suggested Cargo commands

Once crates exist:

```powershell
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
cargo fmt --all -- --check
```

For architecture/model validation:

```powershell
cargo run -p sim-tools -- model validate
```

---

## 82. Compile-time constants vs generated metadata

Static descriptors should use `const`/`static` data where possible.

Avoid heap allocation during registry construction unless needed.

A later build script may generate registry code from declarative metadata, but v0.1 uses native Rust declarations.

---

## 83. Build scripts

`build.rs` may be used for:

- validating static model data;
- generating stable IDs from checked-in manifests;
- generating documentation metadata.

It must not fetch mutable external data during authoritative builds.

Builds should be reproducible offline once dependencies are available.

---

## 84. Stable ID manifest option

A checked-in manifest may reserve stable IDs.

Example:

```text
equations.toml
variables.toml
parameters.toml
```

This can prevent accidental renumbering.

Whether to adopt this immediately is an implementation decision.

---

## 85. Registry order

Canonical registry order:

```text
ascending numeric ID
```

Human-readable listings may sort by key.

Hashes use canonical numeric-ID order.

---

## 86. Registry validation tests

Required tests:

1. no duplicate IDs;
2. no duplicate keys;
3. all output variables exist;
4. all input variables exist;
5. owner consistency;
6. unit compatibility;
7. temporal references are valid;
8. solver groups exist;
9. cycles only occur in declared groups;
10. registry hash is stable.

---

## 87. Unit tests per equation

Every significant equation should have:

- nominal-case test;
- boundary test;
- pathological-input test;
- deterministic repeat test.

Empirical equations should include calibration/reference tests.

---

## 88. Property tests

Good candidates:

```text
stocks never negative after reconciliation
shares remain in bounds
accounting identities balance
zero input produces expected zero/no-change result
higher capacity does not mechanically lower feasible output ceiling
```

Use property-based testing where useful.

---

## 89. Golden model tests

The vertical slice should have golden experiment outputs.

Golden tests should tolerate explicitly defined floating-point tolerance unless state hashing requires exact same-build results.

---

## 90. Performance benchmarks

Hot equation groups should be benchmarked separately.

Candidate tool:

```text
criterion
```

Exact benchmark crate choice is implementation detail.

Benchmarks should include:

- per-country;
- per-sector;
- full-world slice;
- solver iterations.

---

## 91. Registry overhead must remain negligible

Model metadata exists for inspection and validation.

It must not cause every scalar calculation to perform dynamic string/registry lookup.

Hot loops use pre-resolved IDs/indexes and direct Rust functions.

---

## 92. Equation execution handles

A subsystem may compile metadata to runtime handles at startup.

Example:

```rust
pub struct EquationHandle {
    pub id: EquationId,
    pub output_index: usize,
}
```

This avoids repeated registry search.

---

## 93. Causal contribution metadata

Some equations can decompose contributions.

Example:

```text
GDP change:
    consumption contribution
    investment contribution
    trade contribution
```

Such decomposition may be exposed to diagnostics.

It is optional per equation.

---

## 94. Black-box solver tracing

When exact contribution decomposition is not meaningful, causal tracing may record:

```text
these inputs participated
this solver group produced this output
```

Do not invent false precision.

---

## 95. Equation documentation is part of correctness

An equation is not considered complete until its metadata documents:

- purpose;
- owner;
- inputs;
- outputs;
- units;
- cadence;
- temporal semantics;
- provenance;
- solver group if applicable.

---

## 96. Source-code comments are not sufficient

Comments may explain implementation detail.

Registry metadata is the machine-readable source of architecture.

---

## 97. Architecture visualization

A graph exporter should support views:

```text
subsystem graph
equation graph
variable graph
cross-subsystem contract graph
lagged dependency graph
solver-group graph
```

This will help prevent architecture drift.

---

## 98. Large graph filtering

The tooling should support filters by:

- subsystem;
- cadence;
- variable;
- equation class;
- provenance;
- same-tick vs lagged;
- solver group.

This is essential once the model becomes large.

---

## 99. Architecture drift detection

CI should eventually detect:

- new cross-subsystem dependency without contract;
- duplicate writer;
- new cycle outside solver group;
- unit mismatch;
- unregistered authoritative equation.

---

## 100. Consequences

### Positive

- equations become inspectable;
- scheduler metadata is centralized;
- units are validated;
- causal timing is explicit;
- IFs refer-backs remain clean;
- provenance is recorded;
- developer tooling can visualize the model;
- Cargo workspace can validate model structure;
- performance remains native Rust;
- model changes become auditable.

### Costs

- equation metadata adds boilerplate;
- stable IDs require management;
- unit metadata requires discipline;
- registry validation adds startup/tooling complexity;
- proc macros may eventually be desirable;
- model developers must document equations as they add them.

These costs are accepted.

---

## 101. Rejected alternatives

### Generic interpreted equation language as the primary engine

Rejected for v0.1 because it increases complexity and may sacrifice performance, type safety, and debuggability.

### Equations only as ordinary Rust functions with no registry

Rejected because architecture, cadence, units, and causality would become opaque.

### String-only identifiers

Rejected for hot runtime use.

### Automatic hidden registration based on linker/runtime discovery

Rejected for initial implementation because deterministic ordering and tooling should remain obvious.

### Proc-macro-heavy architecture from day one

Deferred because explicit static metadata is easier to validate before macro abstractions stabilize.

---

## 102. Open implementation questions

Still open:

- whether stable IDs are hand-maintained or generated from manifests;
- exact dimensional-analysis representation;
- exact unit-newtype strategy;
- whether metadata uses `&'static str` or interned IDs in release builds;
- exact graph export format;
- exact proc-macro design if added;
- exact benchmark framework;
- exact property-testing crate;
- whether equation documentation is generated to Markdown, JSON, or both.

These do not change the core decision.

---

## 103. Acceptance criteria

ADR-003 is implemented when:

1. `sim-equations` crate exists;
2. stable EquationId/VariableId/ParameterId types exist;
3. unit metadata exists;
4. at least ten equations are registered;
5. all equation inputs/outputs are declared;
6. cadence and temporal semantics are declared;
7. provenance tags are recorded;
8. registry validation passes;
9. one declared causal graph can be exported;
10. the energy-shock vertical slice runs without interpreted equation lookup;
11. `cargo test --workspace` passes;
12. `cargo run -p sim-tools -- model validate` succeeds.

---

# Decision summary

New Engine will use a **compiled Rust equation system with a declarative machine-readable registry**.

Each significant model transformation has:

- stable identity;
- subsystem ownership;
- declared inputs and outputs;
- units;
- variable dimensions;
- cadence;
- temporal/lag semantics;
- solver-group membership;
- provenance;
- model version.

The registry exists for:

- validation;
- scheduling;
- causal tracing;
- documentation;
- model graph generation;
- reproducibility.

The actual equations remain native compiled Rust.

Cargo workspace tooling will make model validation and inspection part of the normal development workflow.

This ADR establishes the semantic model layer that connects New Engine's Rust kernel, subsystem state, and future domain equations.
