# ADR-002: Authoritative State Ownership and Typed Contracts

**Status:** Proposed  
**Date:** 2026-09-17  
**Decision owners:** Project architecture  
**Applies to:** New Engine simulation core  
**Related:** `ADR-001-deterministic-discrete-time-kernel.md`, `../SYSTEM_ARCHITECTURE_V0_1.md`

## Context

ADR-001 established a Rust deterministic discrete-time simulation kernel with:

- a monthly authoritative master tick for the initial modern-world profile;
- subsystem-specific cadences;
- immutable committed state during calculation;
- typed cross-subsystem contracts;
- one logical commit point per master tick;
- Jacobi-style outer coupling for tightly connected core systems;
- deterministic replay and state hashing.

The next requirement is to define how authoritative state is represented and protected.

Without a concrete state-ownership model, several failure modes are likely:

- subsystems mutating each other's internals;
- hidden causal dependencies;
- accidental order dependence;
- unclear ownership of derived values;
- duplicated ledgers;
- difficult replay/debugging;
- excessive cloning or borrow conflicts;
- a monolithic `WorldState` that becomes impossible to reason about.

This ADR defines the initial Rust ownership, storage, identity, contract, and mutation rules.

---

## Decision

### 1. Authoritative state is partitioned by subsystem

There will be no single unrestricted mutable world object.

Instead, authoritative simulation state is partitioned into subsystem-owned stores.

Conceptually:

```rust
pub struct WorldState {
    pub demographics: DemographicsState,
    pub economy: EconomyState,
    pub energy: EnergyState,
    pub agriculture: AgricultureState,
    pub governance: GovernanceState,
    pub infrastructure: InfrastructureState,
    pub education: EducationState,
    pub health: HealthState,
    pub environment: EnvironmentState,
    pub human_development: HumanDevelopmentState,
    pub international: InternationalState,
    pub military: MilitaryState,
}
```

This struct is a top-level storage aggregate only.

It does **not** imply unrestricted mutable access.

The scheduler/kernel controls borrowing and mutation so each subsystem can mutate only its own state during its execution step.

---

## 2. One authoritative owner per variable

Every authoritative variable has exactly one owner.

Examples:

```text
Population by cohort           -> Demographics
GDP / value added              -> Economy
Energy production              -> Energy
Food stocks                    -> Agriculture
Tax revenue                    -> Governance
Port capacity                  -> Infrastructure
Enrollment                     -> Education
Morbidity                      -> Health
Water availability             -> Environment
Poverty rate                   -> Human Development
Alliance state                 -> International Relations
Readiness                      -> Military
```

If two subsystems require the same concept, one owns it and the other consumes a contract projection.

Duplicated authoritative ledgers are prohibited unless explicitly justified by a later ADR.

---

## 3. Stable typed IDs

All major entities use stable strongly typed integer IDs.

Example:

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CountryId(pub u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RegionId(pub u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProvinceId(pub u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CohortId(pub u32);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct MilitaryFormationId(pub u32);
```

Rules:

- no raw `u32` IDs at domain boundaries;
- no string identity in hot authoritative paths;
- IDs are stable for the lifetime of a run;
- IDs are serialized explicitly in saves/replays;
- ID allocation is deterministic.

---

## 4. Unit newtypes for important quantities

Important numeric concepts should use unit-safe newtypes.

Example:

```rust
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Population(pub u64);

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Currency(pub f64);

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct EnergyMwh(pub f64);

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct PriceIndex(pub f64);

#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Rate(pub f64);
```

Not every scalar requires a unique wrapper, but quantities that can be accidentally mixed should have distinct types.

Examples that should not share one generic `f64` type:

- energy quantity;
- currency;
- population;
- rate;
- physical capacity;
- price index;
- land area.

---

## 5. Dense storage where the domain is dense

Dense country/region/sector/cohort state should use indexed contiguous storage.

Example:

```rust
pub struct DenseStore<I, T> {
    values: Vec<T>,
    _marker: std::marker::PhantomData<I>,
}
```

Conceptual API:

```rust
impl<I, T> DenseStore<I, T> {
    pub fn get(&self, id: I) -> &T;
    pub fn get_mut(&mut self, id: I) -> &mut T;
}
```

Use cases:

- population by country/cohort;
- GDP by country/sector;
- energy production by country/type;
- agricultural state by country/commodity.

Benefits:

- cache locality;
- deterministic iteration;
- compact memory;
- fast serialization;
- easy parallel partitioning.

---

## 6. Sparse objects may use sparse storage

Sparse entities may use:

- `SlotMap`-style generational handles;
- stable indexed arenas;
- ECS where appropriate;
- sorted maps where sparse keyed access is required.

Candidates:

- military formations;
- infrastructure facilities;
- projects;
- treaties;
- temporary events;
- construction pipelines;
- strategic facilities.

Sparse storage must preserve deterministic iteration semantics when authoritative calculations depend on iteration order.

---

## 7. Committed state is immutable during solve phases

At the beginning of a master tick:

```text
CommittedWorldState(t)
```

is immutable.

Subsystem execution reads from committed state and contract snapshots.

No subsystem mutates committed state in place.

Each subsystem produces next-state data in its own working area.

Conceptually:

```rust
pub struct TickState {
    pub committed: CommittedWorldState,
    pub working: WorkingWorldState,
}
```

The exact implementation may avoid duplicating the entire world.

The semantic rule is what matters:

> Current committed state is read-only until the global commit boundary.

---

## 8. Working state is subsystem-local

Each subsystem owns its own working state during a solve.

Example:

```rust
pub struct EnergyWorkingState {
    pub production: Vec<EnergyMwh>,
    pub demand: Vec<EnergyMwh>,
    pub stocks: Vec<EnergyMwh>,
    pub prices: Vec<PriceIndex>,
}
```

A subsystem may mutate its own working state.

It may not mutate another subsystem's working state.

---

## 9. Cross-subsystem communication uses typed contracts

Contracts are the only normal cross-subsystem data path.

Example:

```rust
pub struct EnergyToEconomy {
    pub production: DenseStore<CountryId, EnergyMwh>,
    pub demand: DenseStore<CountryId, EnergyMwh>,
    pub shortage_index: DenseStore<CountryId, Rate>,
    pub price_index: DenseStore<CountryId, PriceIndex>,
}
```

The contract is a projection of Energy-owned state.

Economy does not receive direct access to `EnergyState`.

---

## 10. Contracts are read-only snapshots

A contract snapshot is immutable after publication for a solve iteration.

Conceptually:

```rust
pub struct ContractSnapshot {
    pub demographics: DemographicsContracts,
    pub economy: EconomyContracts,
    pub energy: EnergyContracts,
    pub agriculture: AgricultureContracts,
    pub governance: GovernanceContracts,
    pub infrastructure: InfrastructureContracts,
}
```

During Jacobi outer iteration, all systems read the same snapshot version.

No subsystem can observe another subsystem's partially updated outputs.

---

## 11. Contract publication is explicit

Each subsystem publishes specific outputs.

Example trait:

```rust
pub trait PublishContracts {
    type Contracts;

    fn publish_contracts(
        &self,
        state: &Self::State,
    ) -> Self::Contracts;
}
```

Publication is not automatic reflection over state.

This forces explicit API design.

---

## 12. Contract fields are semantically stable

A contract field is part of the simulation API.

Changing the meaning of a contract field requires version review.

Example:

```rust
pub struct EnergyToEconomyV1 {
    pub price_index: DenseStore<CountryId, PriceIndex>,
    pub shortage_index: DenseStore<CountryId, Rate>,
}
```

Future incompatible meaning changes should produce:

```text
EnergyToEconomyV2
```

or an equivalent schema/version migration.

---

## 13. Contracts may aggregate internal state

A subsystem can expose a simplified contract.

Example:

Energy may internally track:

```text
crude
gasoline
diesel
jet fuel
natural gas
coal
nuclear
hydro
solar
wind
```

Economy may only need:

```text
aggregate energy price pressure
industrial energy availability
transport fuel availability
electricity shortage
```

The contract is allowed to aggregate.

This helps prevent downstream systems from depending on unnecessary internal detail.

---

## 14. Contracts should minimize coupling

A subsystem should publish what consumers need, not its entire state.

Bad:

```rust
pub struct EnergyContract {
    pub entire_energy_state: EnergyState,
}
```

Preferred:

```rust
pub struct EnergyToMilitary {
    pub diesel_available: DenseStore<CountryId, EnergyMwh>,
    pub jet_fuel_available: DenseStore<CountryId, EnergyMwh>,
    pub electricity_reliability: DenseStore<CountryId, Rate>,
}
```

---

## 15. Consumer-specific contracts are preferred

Instead of one enormous universal contract per subsystem, use consumer-oriented contract families where useful.

Example:

```text
Energy -> Economy
Energy -> Military
Energy -> Environment
Energy -> Governance
```

These contracts may share internal data but expose different interfaces.

This creates narrower dependencies.

---

## 16. Contract derivation must be deterministic

Contract publication must:

- use stable iteration order;
- avoid nondeterministic hash traversal;
- use deterministic aggregation;
- avoid wall-clock data;
- avoid uncontrolled randomness.

Contracts are eligible for hashing in diagnostics.

---

## 17. No cross-subsystem mutable references

Subsystem APIs must not accept mutable references to foreign state.

Forbidden:

```rust
fn solve(
    &mut self,
    energy: &mut EnergyState,
    economy: &mut EconomyState,
);
```

Preferred:

```rust
fn solve(
    &mut self,
    state: &mut EconomyWorkingState,
    inputs: &EconomyInputs,
    ctx: &SolveContext,
);
```

Where `EconomyInputs` contains read-only contracts.

---

## 18. Kernel services are separate from subsystem state

Subsystems may access kernel services through explicit context objects.

Examples:

```rust
pub struct SolveContext<'a> {
    pub tick: Tick,
    pub calendar: &'a SimulationCalendar,
    pub delayed_events: &'a DelayedEventView,
    pub parameters: &'a ParameterStore,
    pub diagnostics: &'a DiagnosticSink,
}
```

Kernel services must not expose arbitrary mutable state.

---

## 19. Exogenous inputs are separate from authoritative state

Scenario inputs and player decisions are represented separately.

Example:

```rust
pub struct ExogenousInputs {
    pub policy_changes: Vec<PolicyChange>,
    pub shocks: Vec<Shock>,
    pub commands: Vec<PlayerCommand>,
}
```

They are applied at defined tick phases.

They are not hidden mutations of subsystem stores.

---

## 20. Parameters are not state

Model parameters are separated from evolving state.

Example:

```rust
pub struct EnergyParameters {
    pub demand_elasticity: f64,
    pub investment_response: f64,
}
```

Parameters may vary by:

- scenario;
- world profile;
- country;
- technology;
- era.

But they are versioned/calibrated inputs, not mutable state unless a model explicitly treats them as endogenous.

---

## 21. Derived caches are not authoritative

Subsystems may cache derived data for performance.

Example:

```rust
pub struct EconomyCache {
    sector_weight_sums: Vec<f64>,
}
```

Rules:

- cache can be discarded/rebuilt;
- cache is not part of authoritative replay semantics;
- cache cannot become an undocumented source of state;
- cache invalidation must be deterministic.

---

## 22. Derived values need explicit status

Every stored variable should conceptually belong to one of:

```text
AuthoritativeState
DerivedState
Parameter
ContractProjection
DiagnosticOnly
CacheOnly
```

This classification should be available in metadata.

---

## 23. State write permissions

The scheduler grants mutation by subsystem.

Conceptual execution:

```rust
match task.subsystem {
    SubsystemId::Energy => {
        let state = world.energy_mut();
        energy_system.solve(state, &contracts, ctx);
    }
    SubsystemId::Economy => {
        let state = world.economy_mut();
        economy_system.solve(state, &contracts, ctx);
    }
    _ => {}
}
```

Actual implementation should avoid a giant `match` if a better static architecture is available, but the ownership restriction remains.

---

## 24. Borrowing model

Prefer splitting world state into independent fields so Rust can statically prove non-overlapping borrows.

Example:

```rust
let WorldState {
    demographics,
    economy,
    energy,
    agriculture,
    governance,
    ..
} = &mut world;
```

Independent subsystem jobs may receive distinct mutable references to their own working stores while sharing immutable contract snapshots.

This is the preferred concurrency model.

---

## 25. Unsafe code policy

`unsafe` is not prohibited globally, but it is not part of the initial architecture.

Initial rule:

> The kernel and subsystem ownership layer should be implemented without `unsafe`.

If performance later requires unsafe optimization, it must:

- be isolated;
- be benchmark justified;
- have tests;
- preserve ownership semantics;
- receive explicit review.

---

## 26. Interior mutability policy

Avoid `RefCell`, `Mutex`, `RwLock`, or atomics as a shortcut around architectural ownership.

They may be used for:

- diagnostics;
- profiling;
- non-authoritative telemetry;
- carefully isolated concurrent infrastructure.

They should not be the default mechanism for authoritative simulation state.

---

## 27. Stable iteration order

Authoritative iteration order must be defined.

For dense stores:

```text
ascending stable ID
```

For sparse stores:

```text
ascending stable key
```

unless an explicitly different order is part of the model.

Do not rely on insertion order accidentally.

---

## 28. Registry model

Static registries define stable ID spaces.

Example:

```rust
pub struct WorldRegistry {
    pub countries: CountryRegistry,
    pub regions: RegionRegistry,
    pub commodities: CommodityRegistry,
    pub energy_types: EnergyTypeRegistry,
    pub sectors: SectorRegistry,
}
```

Registries are constructed during world load and then treated as immutable.

---

## 29. Registry IDs and save compatibility

Registry IDs are serialized.

A save file must not assume IDs can be regenerated from load order unless the data version explicitly guarantees that.

Recommended:

```text
stable external key
+
runtime numeric ID
```

Example:

```rust
pub struct CountryRecord {
    pub id: CountryId,
    pub key: CountryKey,
}
```

Where `CountryKey` may be a stable string/interned identifier used for migration and data mapping.

---

## 30. Entity deletion policy

Dense world entities such as countries should normally persist.

Sparse entities may be removed.

If IDs can be reused, generational handles are required.

For historical traceability, important strategic entities may instead use tombstones.

Example:

```text
MilitaryFormation:
    Active
    Destroyed
    Disbanded
```

rather than immediate ID reuse.

---

## 31. Ownership of shared physical objects

Some objects affect multiple systems.

Example:

A refinery is relevant to:

- Infrastructure;
- Energy;
- Economy;
- Military;
- Environment.

Ownership must still be singular.

Initial rule:

> Physical facilities are owned by the subsystem responsible for their physical capacity/state.

For a refinery:

```text
Infrastructure or Energy
```

must be chosen once in the relevant domain ADR.

Other systems consume contracts.

No facility should have separate authoritative hit-point/capacity records in several systems.

---

## 32. Ownership of money flows

A money flow also needs one owner.

Example:

Military procurement:

```text
Governance owns authorized/actual budget expenditure
Military owns physical procurement requirement and delivered equipment
Economy owns industrial production/output
```

Contracts connect the three.

This prevents duplicate accounting.

---

## 33. Ownership of population effects

Example:

Combat casualty event:

```text
Military determines military casualty event
Demographics owns authoritative population reduction
Health may own morbidity consequences
Governance consumes political/fiscal effects
```

Military does not directly decrement population storage.

Instead it publishes casualty flows.

---

## 34. Ownership of infrastructure damage

If Infrastructure owns physical infrastructure:

```text
Military publishes damage events
Infrastructure applies authoritative capacity damage
Economy consumes resulting throughput/service loss
```

This preserves a single physical ledger.

---

## 35. State transition pattern

Preferred pattern:

```text
Inputs
 + committed owned state
 + contract snapshot
 + due events
 -> subsystem solve
 -> owned pending state
 -> published contracts
 -> reconciliation
 -> commit
```

This is the standard causal shape.

---

## 36. Contract build phases

Contracts are built at explicit synchronization barriers.

Initial barriers:

```text
Start-of-tick inherited contracts
Core iteration contracts
Post-core contracts
Post-strategic contracts
End-of-tick committed contracts
```

Not every subsystem needs to publish at every barrier.

---

## 37. Contract snapshot version IDs

Each contract snapshot should carry:

```rust
pub struct ContractSnapshotId {
    pub tick: Tick,
    pub phase: TickPhase,
    pub iteration: u32,
}
```

This improves traceability.

---

## 38. Contract validation

At startup or world load, validate:

- required publishers exist;
- required fields exist;
- units are compatible;
- consumer cadence is compatible;
- no illegal same-phase mutation dependency exists;
- all contract versions are supported.

Failure is explicit.

---

## 39. Optional inputs

Optional cross-system inputs must be represented explicitly.

Example:

```rust
pub struct EconomyInputs<'a> {
    pub demographics: &'a DemographicsToEconomy,
    pub energy: &'a EnergyToEconomy,
    pub agriculture: Option<&'a AgricultureToEconomy>,
}
```

Do not use silent defaults for missing required contracts.

---

## 40. Contract units

Contract fields should use typed units.

Bad:

```rust
pub energy: f64
```

Better:

```rust
pub industrial_energy_available: EnergyMwh
```

Where dimensional arrays are needed:

```rust
DenseStore<CountryId, EnergyMwh>
```

---

## 41. Contract temporal semantics

A field must be understood as one of:

```text
instantaneous state
period average
period total flow
end-of-period stock
beginning-of-period stock
rate
index
```

This should be documented and eventually represented in metadata.

Example:

```text
Monthly fuel consumption -> period total flow
Fuel stocks              -> end-of-period stock
Unemployment rate        -> period average or end-period, explicitly defined
```

---

## 42. No hidden global singleton state

The simulation core must not depend on mutable global singleton state.

Allowed globals:

- compile-time constants;
- immutable static tables;
- logger handles that do not affect authoritative state.

Authoritative world state must be passed through explicit ownership/context.

---

## 43. No subsystem-to-subsystem service calls during solve

During solve phases, a subsystem should not call another subsystem's model logic directly.

Forbidden conceptual pattern:

```text
Economy::solve()
    -> Energy::calculate_price()
```

Preferred:

```text
Energy publishes contract
Economy consumes contract
```

The scheduler owns execution order.

---

## 44. Exceptions for pure shared libraries

Pure stateless helper libraries may be shared.

Examples:

- math functions;
- interpolation;
- unit conversion;
- deterministic sampling;
- optimization primitives.

These are not subsystem calls.

---

## 45. Cross-cutting accounting systems

Some domains may appear cross-cutting, such as:

- national accounting;
- trade;
- finance;
- logistics.

They must still have explicit ownership.

If one becomes too broad to fit an existing subsystem, it should become its own subsystem rather than becoming hidden global state.

---

## 46. Contract evolution policy

Contract changes fall into:

### Additive compatible

Example:

```text
add optional diagnostic field
```

Can retain same version where safe.

### Semantic compatible

Example:

```text
performance optimization with unchanged meaning
```

No version bump required.

### Breaking

Example:

```text
change units
change aggregation meaning
rename with semantic change
remove required field
```

Requires version change or migration.

---

## 47. Serialization policy

Contracts are runtime interfaces.

They do not automatically become save-file schemas.

Authoritative subsystem state has separate serialization schemas.

This prevents save compatibility from freezing every internal contract forever.

---

## 48. Testing state ownership

Tests must verify:

- no duplicate authoritative owner for registered variable IDs;
- contract-only dependency paths;
- no unauthorized mutable access;
- deterministic contract publication;
- contract unit consistency;
- stable ID iteration.

---

## 49. Compile-time enforcement goals

Where practical, use the Rust type system to make illegal states difficult to express.

Examples:

- private subsystem state fields;
- public read-only accessors;
- typed contract structs;
- unit newtypes;
- subsystem-specific IDs;
- distinct committed vs working state types.

Avoid relying only on comments.

---

## 50. Proposed crate boundaries

Initial direction:

```text
crates/
  sim-types/
      IDs
      units
      time
      common enums

  sim-state/
      WorldState
      registries
      state metadata
      serialization interfaces

  sim-contracts/
      cross-subsystem contract types
      contract versioning
      contract snapshots

  sim-kernel/
      scheduler
      borrowing/orchestration
      commit
      delayed events
      replay/hash

  subsystem-*/
      owned state
      model logic
      contract publication
```

Dependency rule:

```text
sim-types
    <- sim-state
    <- sim-contracts
    <- subsystem crates
    <- sim-kernel orchestration
```

Exact Cargo dependency direction may be refined to avoid cycles, but subsystem implementation crates must not directly depend on each other.

---

## 51. Subsystem crate dependency rule

Forbidden:

```text
subsystem-economy
    -> subsystem-energy
```

Preferred:

```text
subsystem-economy
    -> sim-contracts
subsystem-energy
    -> sim-contracts
```

The kernel composes both.

---

## 52. Example Energy subsystem

```rust
pub struct EnergyState {
    production: DenseStore<CountryEnergyId, EnergyMwh>,
    demand: DenseStore<CountryEnergyId, EnergyMwh>,
    capacity: DenseStore<CountryEnergyId, EnergyCapacity>,
    stocks: DenseStore<CountryEnergyId, EnergyMwh>,
    prices: DenseStore<CountryEnergyId, PriceIndex>,
}

pub struct EnergySystem {
    parameters: EnergyParameters,
}

pub struct EnergyInputs<'a> {
    pub economy: &'a EconomyToEnergy,
    pub infrastructure: &'a InfrastructureToEnergy,
    pub environment: &'a EnvironmentToEnergy,
    pub governance: &'a GovernanceToEnergy,
    pub military: Option<&'a MilitaryToEnergy>,
}
```

Only Energy code receives mutable access to `EnergyState`.

---

## 53. Example casualty propagation

```text
Military solve
    -> produces MilitaryCasualtyFlow
    -> publish MilitaryToDemographics

Demographics solve
    -> applies casualty flow
    -> updates population/cohort state

Demographics publish
    -> new manpower/population contracts

Economy / Governance / Military
    -> consume next synchronized snapshot
```

This demonstrates causal ownership without duplicate state.

---

## 54. Example refinery strike propagation

```text
Shock:
    refinery physical capacity damaged

Owner:
    Energy or Infrastructure, per facility ownership ADR

Flow:
    capacity owner updates physical capacity
    -> Energy availability contract
    -> Economy production/price response
    -> Military fuel availability response
    -> Governance fiscal response
    -> Environment emissions response
```

No subsystem directly edits GDP, readiness, tax revenue, or emissions as part of the original strike event.

---

## 55. Consequences

### Positive

- ownership is explicit;
- Rust borrowing can enforce many boundaries;
- subsystem APIs remain narrow;
- causality is easier to trace;
- deterministic parallelism is easier;
- duplicate ledgers are reduced;
- model replacement is easier;
- contracts become testable integration surfaces;
- storage can remain data-oriented internally.

### Costs

- more contract structs;
- explicit synchronization barriers;
- some duplicated projection data;
- additional schema/version work;
- subsystem integration requires more design discipline;
- certain tightly coupled equations may need careful contract iteration.

These costs are accepted.

---

## 56. Rejected alternatives

### Monolithic mutable `WorldState`

Rejected because it allows arbitrary mutation and obscures ownership.

### Direct subsystem dependencies

Rejected because they create hidden scheduling and architectural coupling.

### One universal mega-contract

Rejected because it exposes too much state and increases coupling.

### Everything in ECS

Rejected because dense numeric world-state arrays do not necessarily benefit from ECS.

### Everything in `HashMap`

Rejected because of cache cost and iteration-order risk.

### Shared authoritative mirrors

Rejected because they create reconciliation problems and ambiguous ownership.

---

## 57. Open implementation questions

The following remain open:

- exact dense-store generic implementation;
- whether `slotmap`, `generational-arena`, or custom arenas are used for sparse objects;
- exact unit/newtype library strategy;
- whether contract storage uses owned values, `Arc`, or borrowed views at specific barriers;
- exact serialization library;
- exact world-registry format;
- facility ownership boundary between Energy and Infrastructure;
- trade subsystem ownership;
- financial subsystem ownership;
- whether some contract fields use copy-on-write for large arrays.

These are implementation questions, not changes to the ownership model.

---

## 58. Acceptance criteria

ADR-002 is implemented when:

1. Rust types exist for stable IDs and core units;
2. `WorldState` is partitioned by subsystem;
3. subsystem state fields are not globally mutable;
4. Economy and Energy exchange data only through typed contracts;
5. a test proves Economy cannot mutate Energy state through normal APIs;
6. deterministic contract snapshots exist;
7. contract publication order is stable;
8. one controlled energy-shock experiment runs through the ownership model;
9. authoritative state hashes remain stable across repeated runs.

---

## Decision summary

New Engine will use **single-owner authoritative subsystem state** with **typed, immutable, versioned cross-subsystem contracts**.

Rust's ownership system will reinforce the architecture:

- one authoritative owner per variable;
- no direct foreign-state mutation;
- stable typed IDs;
- unit-safe numeric newtypes;
- dense indexed stores for dense world data;
- sparse deterministic storage for sparse entities;
- explicit contract publication;
- consumer-oriented narrow interfaces;
- immutable contract snapshots at synchronization barriers;
- separate committed and working state semantics;
- kernel-controlled mutation and commit.

This becomes the data-ownership foundation for all subsequent subsystem and equation work.
