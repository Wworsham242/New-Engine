# ADR-009: ECS Scope, Dense Subsystem Storage, and Data-Oriented Layout

**Status:** Proposed  
**Date:** 2026-09-17  
**Decision owners:** Project architecture  
**Applies to:** New Engine runtime storage, subsystem data layout, sparse entities, hot loops  
**Related:** `ADR-001-deterministic-discrete-time-kernel.md`, `ADR-002-authoritative-state-ownership-and-typed-contracts.md`, `ADR-003-equation-registry-units-cadence-and-provenance.md`, `ADR-004-solver-groups-convergence-and-numerical-stability.md`, `ADR-005-delayed-effects-pipelines-buffers-and-expectations.md`, `ADR-008-military-operational-substeps-force-abstraction-and-civilian-synchronization.md`, `../SYSTEM_ARCHITECTURE_V0_1.md`

## Context

New Engine combines two very different kinds of simulation state.

One class is dense, regular, and highly numeric:

- population by country/cohort;
- GDP by country/sector;
- energy production by country/type;
- agriculture by country/commodity;
- fiscal accounts;
- prices;
- trade matrices;
- environmental state;
- demographic cohorts.

The other class is sparse, object-like, and heterogeneous:

- military formations;
- air wings;
- naval task groups;
- facilities;
- infrastructure nodes;
- projects;
- construction pipelines;
- events;
- treaties;
- strategic assets;
- temporary operational missions.

A single universal storage model is unlikely to fit both efficiently.

Using ECS for all data would add indirection and component bookkeeping to dense numerical state.

Using only monolithic arrays would make sparse lifecycle-heavy entities awkward.

This ADR defines a hybrid data-oriented architecture.

---

# Decision

## 1. New Engine uses hybrid storage

The engine will use:

```text
Dense indexed storage
+
Sparse entity storage
```

There is no requirement that all authoritative state use ECS.

---

## 2. Dense subsystem state uses contiguous indexed arrays

Dense world-state data should prefer:

- `Vec<T>`;
- structure-of-arrays;
- flat multidimensional arrays;
- compact stable integer indexing.

Examples:

```text
Country x Sector GDP
Country x EnergyType production
Country x AgeCohort population
Country x Commodity inventory
```

---

## 3. Stable typed IDs index dense stores

Example:

```rust
pub struct CountryId(pub u32);
pub struct SectorId(pub u16);
pub struct EnergyTypeId(pub u16);
```

Dense storage maps IDs to contiguous positions.

---

## 4. DenseStore abstraction

Conceptual:

```rust
pub struct DenseStore<I, T> {
    values: Vec<T>,
    _marker: PhantomData<I>,
}
```

The exact implementation may use:

- direct newtype-to-index conversion;
- checked indexing in debug;
- unchecked optimized indexing only if later justified.

---

## 5. Multidimensional dense stores are flattened

Example:

```text
Country x Sector
```

maps to:

```text
index = country * sector_count + sector
```

This avoids nested vector allocation and improves cache locality.

---

## 6. SoA is preferred for hot numeric state

For hot loops, prefer:

```text
Vec<production>
Vec<demand>
Vec<price>
Vec<capacity>
```

over:

```text
Vec<LargeStruct { production, demand, price, capacity, ... }>
```

when equations consume fields independently.

---

## 7. AoS is acceptable where records are consumed together

If an entity's fields are usually accessed as a unit, array-of-structs may be appropriate.

Storage decisions should be benchmark-driven.

---

## 8. Dense data belongs to subsystem owners

Example:

```text
Economy owns dense economic arrays
Energy owns dense energy arrays
Demographics owns dense cohort arrays
```

No shared mutable central data slab.

---

## 9. Sparse entities use stable arena/ECS-like storage

Sparse lifecycle-heavy objects may use:

- generational arenas;
- slot-map style handles;
- ECS;
- sparse sets.

---

## 10. ECS is justified for compositionally sparse entities

Good candidates:

```text
Military formations
Facilities
Infrastructure nodes
Projects
Temporary missions
Strategic assets
Map objects
Events
```

where not every entity has every component.

---

## 11. ECS is not the default for dense numerical matrices

Do not represent:

```text
every country-sector pair
```

as an ECS entity merely for architectural uniformity.

This would create excessive entity/component overhead.

---

## 12. Sparse entity identity is stable

Use generational/stable handles where deletion/reuse exists.

Conceptual:

```rust
pub struct EntityHandle {
    pub index: u32,
    pub generation: u32,
}
```

---

## 13. Strategic historical entities may use tombstones

Important entities such as:

- military formations;
- named ships;
- major facilities;

may retain identity after destruction/disbandment.

This supports replay and history.

---

## 14. ECS components must remain domain-owned

An ECS is storage infrastructure, not an excuse to dissolve subsystem ownership.

Military components remain owned by Military.

Infrastructure components remain owned by Infrastructure.

---

## 15. Cross-subsystem access still uses contracts/events

A facility ECS entity is not globally mutable by every system.

Other subsystems consume projections/contracts.

---

## 16. Hot loops avoid dynamic trait-object dispatch

Sparse entity processing should prefer:

- static component queries;
- stable archetype/sparse-set iteration;
- direct typed systems.

---

## 17. ECS library choice is not fixed by this ADR

Potential options:

- `bevy_ecs`;
- `hecs`;
- `shipyard`;
- custom sparse-set/arena.

Selection depends on:

- determinism;
- overhead;
- serialization;
- borrowing model;
- no-GPU/headless support;
- benchmark results.

---

## 18. Game engine ECS is not automatically simulation ECS

If the UI later uses Godot/Bevy/etc., its ECS/node model does not become authoritative simulation storage automatically.

Simulation core remains independent.

---

## 19. Headless simulation is mandatory

Storage architecture must support:

```text
cargo run / tests / benchmark
```

without graphics engine dependency.

---

## 20. Dense and sparse IDs must interoperate

Example:

```text
Facility entity
 -> CountryId
 -> ProvinceId
 -> EnergyTypeId
```

Sparse objects reference dense registry IDs through typed handles.

---

## 21. Sparse entities may project into dense aggregates

Example:

```text
all refineries in Country X
 -> aggregate refinery capacity
```

Energy can cache/recompute dense aggregate views.

---

## 22. Aggregate projections are derived state

If aggregate capacity can be recomputed from facilities, classify it as derived/cache unless it is explicitly authoritative for performance reasons.

---

## 23. If aggregates are authoritative, one owner remains

No duplicate authoritative facility and aggregate ledgers unless reconciliation semantics are explicit.

---

## 24. Dense arrays are canonical for country-scale simulation

Country/sector/cohort calculations should batch over contiguous arrays.

This is the main performance path for IFs-like systems.

---

## 25. Sparse simulation scales with active entities

Military and infrastructure work should scale primarily with:

- active formations;
- relevant facilities;
- active projects;
- active theaters.

---

## 26. Inactive sparse entities can be cadence-gated

Example:

A reserve formation may only update monthly or quarterly.

An active combat formation updates weekly.

---

## 27. Archetype explosion should be avoided

If using ECS, do not model every tiny state flag as component presence/absence if it causes excessive archetype fragmentation.

Some state belongs in enums/bitflags.

---

## 28. Component granularity should reflect access patterns

Bad:

```text
100 tiny components always accessed together
```

Better:

```text
cohesive component groups
```

if they are updated together.

---

## 29. Sparse set ordering must be deterministic

If authoritative calculations depend on iteration order:

```text
sort by stable ID
```

or use deterministic component storage.

---

## 30. Do not trust insertion order implicitly

Insertion timing can vary after refactors.

Authoritative reductions require explicit ordering.

---

## 31. Dense reductions use fixed index order

Example:

```text
sum sectors in ascending SectorId
```

This supports deterministic floating-point behavior.

---

## 32. Parallel dense loops use deterministic reduction

Independent outputs can be parallel.

Global sums use ordered reduction.

---

## 33. Chunking for parallelism

Dense arrays may be processed in fixed-size chunks.

Example:

```text
country ranges
sector ranges
```

Results merge deterministically.

---

## 34. NUMA/GPU are future concerns

ADR-009 does not require GPU compute or NUMA-specific placement.

Data-oriented layout should keep those options open.

---

## 35. Sparse spatial indexing

Military/infrastructure entities may use separate spatial indexes:

- grid buckets;
- province lists;
- theater lists;
- route-node indexes.

Spatial index is derived acceleration data unless future behavior depends on it.

---

## 36. Spatial index rebuild must be deterministic

Rebuild from authoritative entity locations.

---

## 37. Map raster data is dense

Large world rasters such as:

- terrain;
- elevation;
- precipitation;
- soil;
- land use;

should use dense tiled arrays, not ECS entities per cell.

---

## 38. Province metadata may be dense registry records

If province count is bounded/stable, use indexed arrays.

---

## 39. Infrastructure graph uses sparse graph storage

Road/rail/port/power networks may use:

- node arrays;
- edge arrays;
- adjacency lists;
- compressed sparse graph layouts.

This is not necessarily ECS.

---

## 40. Graph IDs are stable typed IDs

Example:

```rust
pub struct InfrastructureNodeId(pub u32);
pub struct InfrastructureEdgeId(pub u32);
```

---

## 41. Network algorithms operate on compact graph arrays

Routing, flow, and connectivity should avoid object-heavy pointer graphs.

---

## 42. Projects/pipelines may use sparse arenas

Construction and procurement projects are lifecycle-heavy and sparse.

Good fit for arena storage.

---

## 43. Delayed-event queue remains separate

Do not force scheduled events into ECS if a priority queue/timing wheel is more efficient.

---

## 44. Storage follows domain access pattern, not ideological consistency

Use the simplest high-performance representation for each state class.

---

## 45. No universal EntityId

Avoid making every domain concept a generic entity if typed IDs are clearer.

Examples:

```text
CountryId
FormationId
FacilityId
PipelineId
```

are preferable to one opaque global `EntityId` in many cases.

---

## 46. Global entity identity only where necessary

If cross-domain tooling needs universal identity, add a tagged reference:

```rust
pub enum WorldObjectRef {
    Formation(FormationId),
    Facility(FacilityId),
    Pipeline(PipelineId),
}
```

---

## 47. Serialization respects storage semantics

Dense arrays serialize in canonical index order.

Sparse entities serialize in stable ID order.

---

## 48. Save format is independent of in-memory library choice

Switching ECS library should not inherently break saves.

Persistent schema is logical, not raw component storage.

---

## 49. Snapshot hashing follows logical canonical order

Never hash raw ECS memory pages.

---

## 50. ECS queries are not persistence schemas

Persistence explicitly maps authoritative components to stable schema.

---

## 51. Component versioning

If ECS components persist, they require schema/version handling like other subsystem state.

---

## 52. Dense state metadata remains registered

ADR-003 variable registry describes dense variables.

Storage layout is implementation detail.

---

## 53. Sparse component metadata may also be registered

Useful for tooling and save inspection.

---

## 54. Memory layout profiling is required

Use benchmarks to determine:

- SoA vs AoS;
- cache misses;
- allocation count;
- entity iteration cost;
- serialization cost.

---

## 55. Performance baseline

The engine should maintain headless benchmarks for:

```text
dense country-sector updates
cohort updates
energy updates
active military formations
infrastructure graph updates
```

---

## 56. ECS overhead budget

If ECS query/storage overhead becomes material relative to domain calculations, replace or narrow ECS usage.

No architectural loyalty to a library.

---

## 57. Reserve capacity

Dense stores may allocate fixed registry-sized capacity at world load.

This avoids repeated reallocations.

---

## 58. Sparse arenas can reserve estimated capacity

Formation/facility counts are far lower than dense matrix cells.

---

## 59. Memory budgets are profile-dependent

High-fidelity profiles may use:

- more sectors;
- more commodities;
- more equipment categories;
- finer geography.

Storage abstractions should scale by profile.

---

## 60. Data locality between contracts and state

Frequently published contract fields may be stored/derived in contiguous structures.

---

## 61. Contract snapshots should avoid deep clones where possible

Options:

- immutable borrowed views;
- `Arc` to immutable arrays;
- double-buffered owned arrays;
- selective copy of bridge variables.

Exact approach depends on borrow/concurrency design.

---

## 62. Double-buffering is preferred for dense next-state updates

Example:

```text
current[]
next[]
swap()
```

This fits committed/pending semantics.

---

## 63. Full-world clone per tick is discouraged

Do not clone entire `WorldState` merely to achieve immutability if per-subsystem double buffers can avoid it.

---

## 64. Copy-on-write may be useful for experiment branching

Baseline/shock branches can share immutable pages/state until divergence.

This is future optimization.

---

## 65. Sparse entity mutation remains local

During a subsystem phase, mutate only owned arena/ECS stores.

---

## 66. Cross-subsystem object references must be stable

If Military references an airfield:

```text
InfrastructureFacilityId
```

not pointer/reference into Infrastructure storage.

---

## 67. Dangling reference checks

If facilities can be destroyed/deleted, references use:

- tombstones;
- generations;
- validation.

---

## 68. Prefer tombstones for strategically referenced infrastructure

Destroying a bridge usually changes its status rather than erasing identity.

---

## 69. Formation destruction similarly retains historical identity

State:

```text
Active
Destroyed
Disbanded
```

rather than immediate ID reuse.

---

## 70. Dense stores rarely delete IDs

Countries/sectors/cohorts remain stable.

Inactive entries can use status flags.

---

## 71. Component bitsets

Sparse-set/ECS implementations may use bitsets for fast filtering.

This is acceptable if iteration ordering is normalized for authoritative work.

---

## 72. Simulation scheduler sees tasks, not storage internals

Kernel schedules subsystem jobs.

Subsystem owns whether state is:

- dense arrays;
- arena;
- ECS;
- graph.

---

## 73. Storage abstraction boundaries

Do not expose raw ECS world or raw vectors across subsystem boundaries.

Expose typed APIs/contracts.

---

## 74. Example Economy storage

```rust
pub struct EconomyState {
    pub gdp_by_sector: Dense2<CountryId, SectorId, RealGdp>,
    pub employment_by_sector: Dense2<CountryId, SectorId, Employment>,
    pub prices_by_sector: Dense2<CountryId, SectorId, PriceIndex>,
}
```

---

## 75. Example Demographics storage

```rust
pub struct DemographicsState {
    pub population:
        Dense3<CountryId, AgeBandId, SexId, Population>,
}
```

---

## 76. Example Military storage

```rust
pub struct MilitaryState {
    pub formations: FormationArena,
    pub air_wings: AirWingArena,
    pub task_groups: NavalTaskGroupArena,
    pub theaters: TheaterArena,
}
```

Could later use ECS internally.

---

## 77. Example Infrastructure storage

```rust
pub struct InfrastructureState {
    pub nodes: Vec<InfrastructureNode>,
    pub edges: Vec<InfrastructureEdge>,
    pub facilities: FacilityArena,
}
```

---

## 78. Example pipeline storage

```rust
pub struct PipelineStore {
    pub entries: PipelineArena,
    pub due_index: TimingIndex,
}
```

---

## 79. Dense matrix helper crates/modules

Possible shared abstractions:

```text
sim-storage/
    Dense1
    Dense2
    Dense3
    StableArena
    canonical iteration helpers
```

Keep them lightweight.

---

## 80. Avoid generic abstraction tax

Do not create overly generic dimensional containers if simple flat vectors benchmark better and are clearer.

---

## 81. Unsafe optimization policy remains ADR-002

Initial storage implementation should avoid unsafe.

Introduce unsafe only after profiling and isolated review.

---

## 82. SIMD opportunity

Dense SoA loops may later use SIMD.

Layout should keep this possible.

---

## 83. GPU opportunity

Large raster/environmental calculations may later use GPU/compute.

Authoritative determinism implications require separate ADR before adoption.

---

## 84. Sparse-to-dense aggregation

Example:

```text
facility capacities
 -> country energy capacity
```

Aggregation order is stable by facility ID.

---

## 85. Dense-to-sparse allocation

Example:

```text
national fuel allocation
 -> military theater/formation deliveries
```

Allocation uses explicit deterministic policy.

---

## 86. Ownership stays singular through projections

A projection is not a second authoritative copy.

---

## 87. Cached projections have invalidation rules

Example:

Facility change increments an owner-local version counter.

Aggregate view recomputes when stale.

---

## 88. Version counters are non-semantic unless behavior depends on them

Do not hash pure cache version counters.

---

## 89. Archetype migration cost

If using ECS, frequent add/remove component churn can be expensive.

Use flags/state enums where churn is routine.

---

## 90. Military mission assignment may be component/state

If mission changes every week, a field may be better than moving archetypes constantly.

---

## 91. Projects have heterogeneous data

Different project types may use:

- enum payloads;
- typed arenas per project family;
- ECS components.

Benchmark before choosing.

---

## 92. Event payloads use enums

Delayed event queue benefits from compact enums over heavyweight dynamic objects.

---

## 93. No Box<dyn Event> in hot authoritative queue by default

Prefer tagged enum payloads or registered compact representations.

---

## 94. Storage and provenance

Storage changes do not alter equation/model provenance unless semantics change.

---

## 95. Storage and replay compatibility

Changing in-memory layout should not change state hashes if logical state is unchanged.

This is a test requirement.

---

## 96. Layout migration test

Refactor storage, then verify golden replay hashes remain identical.

---

## 97. Deterministic iteration helper

Provide utilities such as:

```rust
for id in registry.country_ids() {
    ...
}
```

with guaranteed order.

---

## 98. Debug bounds checking

Development builds should use checked access and ownership assertions.

---

## 99. Release optimization

Release can remove redundant checks only if correctness is covered by validation/tests.

---

## 100. Memory diagnostics

`sim-tools` should eventually report:

```text
bytes by subsystem
bytes by store
entity counts
dense matrix dimensions
capacity vs used
```

---

## 101. Storage benchmark CLI

Potential:

```powershell
cargo bench -p sim-storage
```

---

## 102. ECS candidate benchmark

Compare:

```text
custom arena
bevy_ecs
hecs
```

for:

- 10k formations/facilities;
- query/update;
- serialization;
- insertion/removal.

---

## 103. Decision criterion for ECS adoption

Choose ECS if it materially improves:

- sparse composition;
- lifecycle management;
- query ergonomics;
- scheduling;

without unacceptable:

- overhead;
- determinism complexity;
- serialization burden.

---

## 104. No requirement to expose ECS to game/mod APIs

Mods/scenario tools can operate on higher-level domain interfaces.

---

## 105. Data-oriented subsystem APIs

Subsystem public APIs should accept IDs/slices/views rather than object graphs.

---

## 106. Structure ownership

Registry/layout types may live in shared storage crate.

Actual domain state remains in subsystem crate.

---

## 107. Map rendering representation is separate

UI may build presentation entities/cache from simulation state.

Do not shape authoritative storage around rendering convenience.

---

## 108. Dedicated UI screens benefit from separate representations

Military screen can build task-group/formation views without changing simulation layout.

Economy screen can build sector tables from dense arrays.

---

## 109. Headless-first invariant

Any storage solution that requires renderer/game-engine runtime is rejected.

---

## 110. Initial implementation recommendation

Start with:

```text
Dense flat Vec-based stores
+
custom stable arenas / slotmap-like sparse storage
```

before adopting a full ECS library.

Why:

- simplest deterministic baseline;
- easy serialization;
- low dependency cost;
- exposes actual access patterns;
- avoids premature ECS design.

---

## 111. ECS can be added selectively later

Likely first candidates:

```text
Military formations
Facilities
Temporary missions
```

if benchmarks show benefit.

---

## 112. Initial Cargo workspace impact

Suggested:

```text
crates/
  sim-storage/
```

with:

```text
Dense1
Dense2
Dense3
StableArena
StableId traits
canonical iterators
```

This crate contains no domain logic.

---

## 113. Dependency rule

```text
subsystem-* -> sim-storage
```

`sim-storage` must not depend on subsystem crates.

---

## 114. Acceptance criteria

ADR-009 is implemented when:

1. dense typed stores exist;
2. at least one 2D flat store exists;
3. stable deterministic iteration exists;
4. sparse stable arena exists;
5. Economy/Energy use dense stores;
6. Military formations use sparse stable storage;
7. save/replay schema is independent of in-memory layout;
8. storage refactor test preserves state hashes;
9. headless benchmarks exist;
10. no graphics/ECS runtime dependency is required for simulation.

---

## 115. Consequences

### Positive

- dense IFs-like systems remain cache-efficient;
- military/facility/project entities remain flexible;
- no forced one-size-fits-all ECS architecture;
- headless simulation stays lightweight;
- storage can evolve without changing logical model semantics;
- deterministic iteration is easier to guarantee.

### Costs

- two storage paradigms must be maintained;
- bridge/projection code is required;
- some developers may prefer uniform ECS concepts;
- custom dense helpers require testing.

These costs are accepted.

---

## 116. Rejected alternatives

### ECS for everything

Rejected because dense matrices/cohort systems are better represented as arrays.

### One giant struct-of-structs object graph

Rejected for cache locality and ownership reasons.

### HashMap-based storage everywhere

Rejected for determinism and performance.

### Renderer/game-engine ECS as authoritative simulation state

Rejected because headless independence is mandatory.

### Pure custom storage for every subsystem with no shared helpers

Rejected because stable IDs/deterministic iteration should be standardized.

---

## 117. Open implementation questions

Still open:

- exact sparse arena implementation;
- whether a full ECS library is eventually adopted;
- SoA vs AoS choices per subsystem;
- exact dense multidimensional API;
- copy-on-write experiment branching;
- spatial index implementation;
- infrastructure graph representation;
- future GPU/SIMD strategy.

These do not change the core decision.

---

# Decision summary

New Engine will use a **hybrid data-oriented storage architecture**.

Dense system-of-systems state uses:

- contiguous arrays;
- flat multidimensional stores;
- stable typed IDs;
- SoA where hot numeric access benefits.

Sparse lifecycle-heavy objects use:

- stable arenas;
- sparse sets;
- or selective ECS where justified.

ECS is a tool for suitable sparse entities, not the universal architecture.

The simulation remains headless-first, deterministic, and independent of any graphics-engine entity model.
