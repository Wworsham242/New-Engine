# ADR-008: Military Operational Substeps, Force Abstraction, and Civilian Synchronization

**Status:** Proposed  
**Date:** 2026-09-17  
**Decision owners:** Project architecture  
**Applies to:** New Engine military subsystem, simulation kernel, civilian synchronization, force representation  
**Related:** `ADR-001-deterministic-discrete-time-kernel.md`, `ADR-002-authoritative-state-ownership-and-typed-contracts.md`, `ADR-003-equation-registry-units-cadence-and-provenance.md`, `ADR-004-solver-groups-convergence-and-numerical-stability.md`, `ADR-005-delayed-effects-pipelines-buffers-and-expectations.md`, `ADR-006-deterministic-randomness-and-event-sampling.md`, `ADR-007-save-replay-state-hashing-and-compatibility.md`, `../SYSTEM_ARCHITECTURE_V0_1.md`

## Context

New Engine uses a monthly authoritative master tick for the initial modern-world profile.

Military operations need finer temporal resolution than monthly because combat, movement, logistics, readiness, infrastructure damage, fuel expenditure, ammunition expenditure, and operational tempo can change materially within days or weeks.

At the same time, New Engine is not intended to simulate every aircraft, vehicle, missile, sailor, or tactical engagement individually.

The intended military layer is loosely informed by the operational richness of Command: Modern Operations (CMO), but deliberately moved **one abstraction level upward** so that it fits inside an IFs-style integrated world simulation without dominating CPU time or overwhelming the player.

The target is:

```text
CMO-like operational relationships
+
grand-strategy formation abstraction
+
IFs-like civilian/system integration
```

Examples:

```text
Air:
    carrier air wing / land-based air wing / air group
    rather than every individual aircraft

Ground:
    division as the standard large formation
    brigade as an allowed smaller formation
    rather than every battalion/company/vehicle

Naval:
    task group / surface action group / submarine group / amphibious group
    with optional important individual capital platforms inside the aggregate
    rather than every platform and weapon interaction by default
```

The military system must therefore preserve:

- force structure;
- readiness;
- basing;
- mission capability;
- logistics;
- fuel;
- ammunition;
- maintenance;
- attrition;
- command priorities;
- infrastructure dependence;
- procurement and replacement;
- operational geography;

while avoiding a tactical simulation whose computational cost slows the entire world model.

This ADR defines the temporal and abstraction boundary.

---

# Decision

## 1. Military is an operational-strategic subsystem, not a tactical micro-simulator

The default Military model operates one level above individual-platform tactical simulation.

The model should answer questions such as:

```text
Can this carrier group sustain air operations here?
Can this air wing generate enough sorties?
Can this division advance given its fuel, readiness, terrain, and supply?
Can this brigade hold a narrow sector?
Can this task group project sufficient capability into this sea zone?
Can the national industrial system replace current losses?
```

It does not need to simulate every individual sensor sweep, missile seeker, aircraft maneuver, or tank engagement.

---

## 2. CMO is a reference for relationships, not simulation granularity

CMO can inform relationships such as:

- range;
- basing;
- sortie generation;
- weapon availability;
- logistics;
- sensor/intelligence effects;
- air-defense interaction;
- naval reach;
- mission type;
- fuel;
- maintenance;
- platform capability.

New Engine aggregates these relationships into formation-level or package-level state.

The objective is not to reproduce CMO mechanics.

---

## 3. Force echelons are explicit

Initial force abstraction types:

```rust
pub enum FormationScale {
    Brigade,
    Division,
    CorpsAggregate,
    AirWing,
    CarrierAirWing,
    NavalTaskGroup,
    SubmarineGroup,
    AmphibiousGroup,
    StrategicAssetGroup,
}
```

Not every profile must use every echelon.

---

## 4. Ground default is division-scale

The normal maneuver formation for ground forces is:

```text
Division
```

A division represents a structured aggregate of:

- personnel;
- armor;
- artillery;
- air defense;
- engineers;
- logistics;
- support;
- command;
- other assigned capabilities.

---

## 5. Brigades are fully supported

Brigades are smaller formations with fewer subordinate units/capabilities.

They are not a separate simulation paradigm.

A brigade uses the same broad formation model as a division but with:

- lower manpower;
- lower equipment count;
- reduced sustainment capacity;
- reduced frontage/coverage;
- potentially specialized capability;
- lower command overhead.

This is analogous to the practical flexibility of HOI-style formation design without adopting HOI's exact mechanics.

---

## 6. Formation composition matters

A formation is not only a strength scalar.

Conceptual:

```rust
pub struct GroundFormationState {
    pub id: FormationId,
    pub scale: FormationScale,
    pub manpower: PersonnelCount,
    pub armor: EquipmentPool,
    pub artillery: EquipmentPool,
    pub air_defense: EquipmentPool,
    pub logistics: LogisticsCapacity,
    pub engineering: EngineeringCapacity,
    pub readiness: ReadinessState,
    pub training: TrainingState,
    pub supply: SupplyState,
    pub location: OperationalLocation,
}
```

Exact storage may be more data-oriented.

---

## 7. Formation templates are data-driven

Examples:

```text
US armored brigade
mechanized division
light infantry brigade
airborne brigade
marine division
territorial defense brigade
```

Template data defines expected composition.

Actual fielded formation state can deviate due to losses, reinforcement, or reorganization.

---

## 8. Air default is wing/group scale

The normal air entity is:

```text
AirWing
```

or:

```text
CarrierAirWing
```

not an individual aircraft.

An air wing tracks aggregate aircraft inventory by type where needed.

---

## 9. Carrier air wing example

A carrier may expose:

```text
Carrier Air Wing:
    fighters
    strike aircraft
    AEW capability
    electronic warfare
    helicopters
    support
```

Operations are resolved through aggregate:

- aircraft availability;
- sortie generation;
- mission assignment;
- range;
- weapons;
- fuel;
- maintenance;
- losses.

---

## 10. Individual aircraft are inventory, not primary entities

Aircraft count/type may remain explicit numerically.

Example:

```text
48 fighters
5 AEW aircraft
6 EW aircraft
```

But the normal simulation object is the wing.

Losses reduce inventory and therefore future wing capability.

---

## 11. Important exceptional individual platforms may remain explicit

Some strategic assets may justify individual identity:

- aircraft carriers;
- ballistic-missile submarines;
- rare strategic bombers;
- very high-value unique platforms.

Their subordinate routine combat elements can still be aggregated.

This is an exception based on strategic significance, not a requirement to individualize every platform.

---

## 12. Naval default is task-group scale

Primary naval entities may include:

```text
Carrier Strike Group
Surface Action Group
Amphibious Ready Group
Submarine Group
Convoy/Escort Group
```

Composition can track aggregate platform classes.

---

## 13. Naval composition matters

Example task group:

```text
1 carrier
2 cruisers/destroyer leaders
4 destroyers
1 attack submarine support element
logistics support
carrier air wing
```

Capability derives from composition, readiness, weapons inventory, air wing, sensors, and logistics.

---

## 14. Strategic individual ships can be nested within a group

A carrier itself can remain a named asset while escorts are represented as pools/subcomponents.

This supports meaningful carrier loss without forcing every destroyer into a full tactical entity.

---

## 15. Weapon stocks are aggregated

Weapons are tracked at useful operational levels:

```text
air-to-air missile inventory
land-attack missile inventory
anti-ship missile inventory
guided bomb inventory
artillery ammunition
air-defense interceptors
```

not necessarily every weapon serialized as an entity.

---

## 16. Mission packages are temporary operational aggregates

Military can create mission packages such as:

```text
strike package
CAP
SEAD package
amphibious operation
ground offensive
airlift mission
naval patrol
```

They reference participating formations/assets rather than creating permanent duplicated forces.

---

## 17. Theater is a major operational scope

Military activity is organized by theater.

Conceptual:

```rust
pub struct TheaterId(pub u32);
```

A theater contains:

- participating formations;
- logistics network;
- missions;
- command priorities;
- opposing force estimates;
- geographic operational state.

---

## 18. Only active theaters require high-frequency work

Inactive or low-intensity theaters should not run expensive operational calculations each substep.

This is a principal performance strategy.

---

## 19. Military operations use internal substeps

Military may execute multiple operational substeps inside one monthly master tick.

Initial supported profiles:

```text
StrategicMonthly
StrategicWeekly
OperationalDaily
```

Default modern profile:

```text
StrategicWeekly
```

---

## 20. Four strategic weeks per month in v0.1

Initial implementation:

```text
4 equal operational substeps per monthly world tick
```

This is deliberately abstract and computationally predictable.

Daily calendar-aware mode can be added later.

---

## 21. Substeps do not advance world master time

Numerical/operational substeps are internal.

They do not run the whole civilian model repeatedly.

---

## 22. Military owns military state

Military authoritative state includes:

- formations;
- task groups;
- air wings;
- equipment inventories;
- readiness;
- personnel assigned to military formations;
- military ammunition;
- allocated/delivered fuel;
- maintenance state;
- mission state;
- deployment;
- mobilization;
- training;
- military logistics;
- operational control;
- combat losses;
- procurement requirements.

---

## 23. Civilian ownership rules remain binding

Military does not directly own or mutate:

```text
national GDP
civilian population totals
civilian factory production
government fiscal balances
civilian ports/rail/roads
civilian power plants/refineries
```

Military changes them through typed events/contracts.

---

## 24. Operational inputs are mostly frozen within the month

At the start of the operational cycle, Military receives relevant contracts:

- fuel allocation;
- infrastructure availability;
- industrial deliveries;
- basing/access;
- mobilizable manpower;
- budget authorization;
- civilian transport allocation.

These are normally fixed across the four weekly substeps.

---

## 25. Narrow fast synchronization is allowed for physical destruction

Physical events can be too important to defer.

Examples:

- bridge destruction;
- runway destruction;
- port closure;
- refinery destruction;
- power-grid node loss.

These use a fast physical synchronization barrier.

---

## 26. Fast synchronization does not rerun the macro economy

It updates only required physical availability.

Example:

```text
Military strike
 -> Infrastructure applies bridge damage
 -> narrow operational contract rebuilt
 -> Military reroutes next week
```

GDP and tax effects remain monthly.

---

## 27. Combat consequences are causal, never direct macro modifiers

A military strike produces:

- asset damage;
- losses;
- casualties;
- transport interruption;
- stock destruction;
- displacement;
- trade disruption.

It does not produce:

```text
GDP -7%
```

The civilian system computes that outcome.

---

## 28. Military operational sequence

Recommended weekly substep:

```text
1. update orders/mission assignments
2. resolve logistics availability
3. allocate fuel/ammunition
4. movement/deployment
5. air/naval/ground mission resolution
6. combat/attrition
7. update military losses
8. emit physical damage events
9. apply fast physical synchronization
10. update readiness/maintenance/fatigue
11. record substep diagnostics
```

---

## 29. Ground combat is formation-level

Combat inputs may include:

- manpower;
- equipment composition;
- readiness;
- training;
- supply;
- terrain;
- fortification;
- intelligence;
- air support;
- artillery support;
- command;
- operational posture.

The output is aggregate loss/progress/control change.

---

## 30. Ground frontage/coverage can distinguish brigade and division

A brigade may:

- occupy less frontage;
- require fewer supplies;
- be easier to deploy;
- have lower sustained combat mass.

A division may:

- cover larger operational space;
- sustain more complex combined-arms operations;
- require greater logistics.

---

## 31. Smaller units are not free flexibility

Splitting a division into multiple brigades can incur:

- command overhead;
- reduced organic support;
- logistics inefficiency;
- weaker concentration.

Exact mechanics belong to the military model, but scale must matter.

---

## 32. Air operations use sortie capacity

An air wing generates potential sorties based on:

```text
available aircraft
maintenance
crew readiness
fuel
weapons
base condition
distance
mission intensity
```

---

## 33. Air combat can use aggregate mission resolution

Example:

```text
CAP wing
vs
strike package
+
air-defense environment
```

resolve expected/stochastic losses and mission success at package level.

No requirement to simulate each dogfight.

---

## 34. Naval operations use area/task-group resolution

Naval groups operate across:

- sea zones;
- operational areas;
- routes;
- mission areas.

Resolution considers:

- detection;
- air cover;
- missile inventory;
- submarines;
- readiness;
- range;
- logistics;
- weather where modeled.

---

## 35. Detection is abstract but important

CMO-inspired relationships such as detection and sensor advantage should matter.

They are represented at group/mission level.

---

## 36. Basing matters

Air/naval capability depends on:

- airfield/port availability;
- distance;
- tanker/support capacity;
- repair;
- fuel;
- host-nation access.

These connect directly to Infrastructure, Energy, and International Relations.

---

## 37. Range matters without simulating every path

Operational reach can be calculated from:

- base position;
- mission radius;
- refueling;
- basing access;
- transport network.

---

## 38. Readiness is decomposed

Readiness can depend on:

- personnel;
- equipment;
- maintenance;
- fuel;
- ammunition;
- training;
- fatigue;
- supply.

A UI summary score may be derived.

---

## 39. Logistics is a first-class military constraint

Military force that cannot be supplied cannot operate at full effectiveness.

This is one of the main CMO-like relationships preserved at higher abstraction.

---

## 40. Fuel is conserved

Military fuel consumption uses:

```text
starting military fuel
+ allocated/delivered fuel
- operational consumption
```

No negative stocks.

---

## 41. Ammunition is conserved

Same pattern:

```text
starting ammunition
+ deliveries
- combat expenditure
- training expenditure
- losses
```

---

## 42. Supply priority is explicit

National or theater priorities may allocate scarce fuel/ammunition.

No allocation based on accidental iteration order.

---

## 43. Military losses reduce actual composition

Example:

```text
brigade loses 20 tanks
```

Its armor pool changes.

It is not merely assigned a generic temporary combat penalty.

---

## 44. Replacements are delayed

Replacement comes from:

- reserve stocks;
- repair;
- domestic production;
- imports;
- allied aid;
- procurement pipeline.

---

## 45. Procurement is integrated with IFs-like civilian systems

Example:

```text
Military needs 300 vehicles
 -> Governance authorizes spending
 -> Economy allocates industrial production
 -> Energy/inputs constrain output
 -> procurement pipeline advances
 -> Military receives deliveries later
```

This is a defining military/civilian integration pathway.

---

## 46. Casualty synchronization

Formation personnel decline during combat immediately.

At monthly synchronization:

```text
Military casualty flow
 -> Demographics
```

Demographics updates national population/cohorts exactly once.

---

## 47. Damage synchronization

Military creates damage events.

The physical owner applies capacity loss.

This applies to:

- infrastructure;
- energy assets;
- industrial facilities;
- civilian logistics nodes.

---

## 48. Civilian consequences remain on civilian cadence

Military operational changes can be immediate.

Macroeconomic/social effects remain monthly/quarterly/annual as defined by their subsystem.

---

## 49. Monthly outputs

Conceptual:

```rust
pub struct MilitaryMonthlyOutputs {
    pub casualties: MilitaryCasualtyFlow,
    pub equipment_losses: EquipmentLossFlow,
    pub fuel_consumed: FuelQuantity,
    pub ammunition_consumed: AmmunitionQuantity,
    pub procurement_demand: ProcurementDemand,
    pub transport_demand: TransportDemand,
    pub displacement_pressure: DisplacementFlow,
    pub infrastructure_damage: DamageSummary,
    pub operational_control_changes: ControlChangeSet,
}
```

---

## 50. Flow/state aggregation rules

Across weekly substeps:

```text
casualties       -> sum
fuel use         -> sum
ammunition use   -> sum
equipment losses -> sum
readiness        -> end-of-month
formation state  -> end-of-month
operational control -> end-of-month
tempo            -> period average/sum as defined
```

---

## 51. Operational control vs political sovereignty

Military may own operational control.

Governance/International systems determine:

- legal sovereignty;
- recognition;
- administration;
- occupation effects.

This prevents military map state from automatically rewriting political state.

---

## 52. Theater parallelism

Independent active theaters may run in parallel after national resource allocations are fixed.

Shared national resources prevent naive independence.

---

## 53. Deterministic allocation before parallel theater execution

Examples:

- fuel;
- strategic lift;
- replacements;
- long-range missiles;
- national air assets.

Allocation is fixed deterministically before theater jobs execute.

---

## 54. Stable theater ordering

Any required sequential work uses stable IDs or explicit command priority.

---

## 55. Sparse active simulation

Performance principle:

```text
simulate active formations/theaters deeply;
update inactive forces cheaply.
```

Inactive forces may receive lower-frequency:

- maintenance;
- training;
- readiness;
- routine supply.

---

## 56. No global tactical scan

The engine must not check every weapon against every possible target every substep.

Use:

- theater partitioning;
- mission assignments;
- area engagement sets;
- spatial indexing;
- candidate filtering.

---

## 57. Mission-driven work scheduling

Only forces assigned to relevant missions require expensive resolution.

Examples:

```text
strike
defend
patrol
advance
interdict
escort
reserve
```

---

## 58. Data-oriented storage

Formation-level abstraction enables compact contiguous storage.

Potential arrays:

```text
formation readiness
manpower
equipment pools
location
supply
mission
```

rather than millions of individual entities.

---

## 59. Strategic assets can be sparse entities

Rare individual carriers or SSBNs may use sparse entities without compromising overall scale.

---

## 60. Fidelity scaling

Profiles may vary:

```text
force composition resolution
substep cadence
weapon category resolution
sensor/detection complexity
logistics network resolution
damage granularity
```

But contracts remain stable.

---

## 61. StrategicMonthly profile

Lowest military fidelity:

- one military update per world month;
- formation-level attrition;
- aggregate logistics.

Useful for:

- very large worlds;
- peaceful scenarios;
- fast forward.

---

## 62. StrategicWeekly profile

Default:

- four military substeps per month;
- active-theater logistics;
- formation missions;
- fast physical damage sync.

---

## 63. OperationalDaily profile

Higher fidelity:

- calendar-day substeps;
- more detailed mission timing;
- higher CPU cost.

Used selectively.

---

## 64. Dynamic fidelity is possible later

Future optimization may allow peaceful theaters to use monthly cadence while active wars use weekly/daily cadence.

This requires deterministic transition rules.

Not required in v0.1.

---

## 65. CMO-like detail belongs in capability data

Examples:

- range;
- payload;
- weapon type;
- sensor class;
- endurance;
- mission compatibility.

These can inform aggregate force capability without individual simulation.

---

## 66. Equipment catalogs remain detailed

The engine may maintain detailed equipment types:

```text
F-35C
F/A-18E
E-2D
M1A2
Leopard 2A8
Patriot
Aegis destroyer class
```

but groups them in formations/air wings/task groups.

---

## 67. Composition can affect mission eligibility

Example:

A carrier air wing without sufficient AEW/EW assets may have reduced mission capability.

This preserves meaningful combined-system detail.

---

## 68. Force package calculations may be cached

Derived capability vectors can be cached until composition/readiness changes.

This reduces computational cost.

---

## 69. Capability vector

Conceptually:

```rust
pub struct CapabilityVector {
    pub air_superiority: f64,
    pub strike: f64,
    pub air_defense: f64,
    pub anti_ship: f64,
    pub anti_submarine: f64,
    pub ground_attack: f64,
    pub logistics: f64,
    pub reconnaissance: f64,
}
```

This is derived state, not a replacement for composition.

---

## 70. Capability vectors are not arbitrary ratings

They derive from:

- composition;
- readiness;
- inventory;
- support;
- range;
- doctrine;
- conditions.

---

## 71. Mission resolution can consume capability vectors

This allows efficient operational combat without iterating every platform.

---

## 72. Damage can degrade capability nonlinearly

Loss of a small but critical support component can matter disproportionately.

Example:

```text
AEW loss
tanker loss
bridge loss
air-defense radar loss
```

Therefore composition must retain meaningful support categories.

---

## 73. Command and control

C2 quality can affect:

- coordination;
- response;
- mission efficiency;
- information latency.

It may be formation/theater-level state.

---

## 74. Intelligence

Military decisions can eventually consume perceived enemy state.

This is compatible with future authoritative-vs-perceived-state architecture.

---

## 75. Military local solver groups

Potential:

```text
military.logistics
military.air_operations
military.ground_operations
military.naval_operations
military.readiness
```

Use ADR-004 bounded deterministic solvers.

---

## 76. Randomness

If combat includes stochastic realization:

```text
engagement/theater/substep/formation/event key
```

per ADR-006.

No global RNG.

---

## 77. Stochastic draws remain fixed during numerical iteration

Substep solver retries/iterations do not reroll combat.

---

## 78. Save/replay

Standard saves occur at monthly committed boundaries.

Replay regenerates all weekly/daily military substeps deterministically.

---

## 79. Optional military substep hashes

Developer mode can hash after each substep to locate divergence.

---

## 80. Validation: carrier air wing

Test:

```text
carrier group has air wing with finite aircraft, weapons, fuel, maintenance
```

Verify:

- sorties constrained by available aircraft/readiness;
- losses reduce future capability;
- fuel/weapons decline;
- replacement is delayed;
- carrier loss or airfield loss affects capability immediately.

---

## 81. Validation: division vs brigade

Create equal-template family:

```text
1 division
1 brigade
```

Verify:

- brigade has lower total combat mass/sustainment;
- brigade has lower supply consumption;
- both use same underlying formation architecture;
- scale changes capabilities rather than invoking a different rules engine.

---

## 82. Validation: bridge strike

Verify:

- strike generates physical damage event;
- Infrastructure owns bridge state;
- route unavailable in next military substep;
- economic effect appears through normal civilian solve.

---

## 83. Validation: refinery strike

Verify:

- refinery physical capacity drops;
- operational fuel constraints can propagate quickly;
- macro economic effect is monthly;
- no scripted GDP penalty.

---

## 84. Validation: industrial replacement

Heavy losses:

```text
equipment demand rises
```

Verify:

- Governance/Economy must finance/produce replacements;
- deliveries occur later;
- insufficient industrial capacity delays recovery.

---

## 85. Performance target

Military architecture should scale primarily with:

```text
active formations
active theaters
active missions
relevant network nodes
```

not total theoretical platform count.

---

## 86. CPU-budget principle

Military detail must be budgeted so it remains one subsystem among many.

It must not consume most world-simulation time under normal strategic play.

A performance budget will be established by benchmark once implementation exists.

---

## 87. Benchmark scenarios

Required future benchmarks:

```text
peaceful world
one regional war
three simultaneous regional wars
major-power war
maximum supported active formations
```

---

## 88. Cargo crate direction

Likely:

```text
subsystem-military/
    formations
    equipment pools
    theaters
    missions
    logistics
    readiness
    combat
    air
    naval
    ground
    substeps

sim-contracts/
    MilitaryTo*
    *ToMilitary

sim-kernel/
    substep scheduler
    fast physical synchronization
```

---

## 89. No direct civilian subsystem dependencies

Military consumes shared contracts.

Not:

```text
subsystem-military -> subsystem-energy
```

---

## 90. Initial implementation sequence

### Step 1

Create:

```text
FormationId
TheaterId
FormationScale
OperationalSubstep
MilitaryCadenceProfile
```

### Step 2

Implement ground formation state with Division and Brigade scales.

### Step 3

Implement AirWing/CarrierAirWing state.

### Step 4

Implement NavalTaskGroup state.

### Step 5

Implement four-week substep scheduler.

### Step 6

Implement fuel/ammunition/readiness accounting.

### Step 7

Implement mission-driven candidate selection.

### Step 8

Implement minimal aggregate combat/attrition.

### Step 9

Implement physical damage event path.

### Step 10

Connect monthly military outputs to civilian systems.

---

## 91. Acceptance criteria

ADR-008 is implemented when:

1. four deterministic weekly military substeps run inside one monthly tick;
2. a ground Division and Brigade share one formation model but differ by scale/composition;
3. a CarrierAirWing operates as one primary entity with internal aircraft inventory;
4. NavalTaskGroup aggregation exists;
5. no default individual-aircraft simulation is required;
6. fuel/ammunition are conserved;
7. readiness and losses persist;
8. physical damage crosses ownership through events;
9. macro civilian systems do not rerun each substep;
10. procurement/replacement flows through Governance/Economy pipelines;
11. replay reproduces military substeps;
12. active-theater benchmark demonstrates sparse scheduling.

---

## 92. Consequences

### Positive

- military play can feel operationally rich;
- CMO-like dependencies matter without CMO-like entity count;
- air wings, divisions, brigades, and task groups are computationally manageable;
- force composition remains meaningful;
- military logistics integrates naturally with the civilian model;
- military activity does not force the full world to daily cadence;
- dedicated military screens can be rich without crowding the main world view.

### Costs

- aggregate combat models require careful calibration;
- formation composition needs good data;
- support assets must be represented well enough to avoid overly simple force ratings;
- fast physical synchronization adds kernel complexity;
- fidelity profiles require validation.

These costs are accepted.

---

## 93. Rejected alternatives

### Individual aircraft as the default simulation entity

Rejected because CPU/entity cost would be excessive for an integrated world simulation.

### Individual ground vehicles/squads as default

Rejected for the same reason.

### Pure national combat-power scalar

Rejected because it discards logistics, basing, composition, readiness, and operational geography.

### Full CMO-style tactical simulation embedded in every conflict

Rejected because it would dominate computational cost and clash with the system-of-systems design.

### HOI-style divisions with no meaningful operational logistics

Rejected because New Engine requires deeper integration with fuel, infrastructure, industry, and procurement.

### Monthly-only warfare

Rejected because operational consequences can change too much within one month.

---

## 94. Open implementation questions

Still open:

- exact combat equations;
- exact theater geography representation;
- exact equipment-category granularity;
- carrier/task-group nesting model;
- whether Corps exists as true entity or command aggregation only;
- dynamic fidelity switching;
- territory-control ownership;
- fast Energy synchronization scope;
- intelligence/perceived-state design.

These do not change the core decision.

---

# Decision summary

New Engine will use a **CMO-inspired but deliberately higher-level military simulation**.

Default force entities are:

- divisions and optional smaller brigades for ground forces;
- air wings/carrier air wings for aviation;
- naval task groups for fleets;
- selected strategically important platforms individually where justified.

Military operations run in weekly substeps inside the monthly world tick.

The design preserves operational relationships such as:

- readiness;
- basing;
- range;
- mission capability;
- fuel;
- ammunition;
- logistics;
- maintenance;
- support assets;
- infrastructure;
- losses;
- procurement.

It does not simulate every aircraft, vehicle, or weapon as an independent entity by default.

Military action affects the civilian world through explicit physical and resource consequences, allowing the Military subsystem to fit naturally into the broader IFs-like system-of-systems model without slowing the entire simulation.
