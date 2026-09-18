# ADR-005: Delayed Effects, Pipelines, Buffers, and Expectations

**Status:** Proposed  
**Date:** 2026-09-17  
**Decision owners:** Project architecture  
**Applies to:** New Engine simulation kernel and subsystem models  
**Related:** `ADR-001-deterministic-discrete-time-kernel.md`, `ADR-002-authoritative-state-ownership-and-typed-contracts.md`, `ADR-003-equation-registry-units-cadence-and-provenance.md`, `ADR-004-solver-groups-convergence-and-numerical-stability.md`, `../SYSTEM_ARCHITECTURE_V0_1.md`

## Context

ADR-001 established a monthly authoritative master tick with subsystem-specific cadences and internal substeps.

ADR-002 established single-owner authoritative state and typed cross-subsystem contracts.

ADR-003 established explicit equation metadata, temporal semantics, cadence, units, and provenance.

ADR-004 established the rule that same-tick solver iteration should be used only when the causal feedback can reasonably resolve within the current synchronization period.

The complementary requirement is to represent processes that cannot resolve instantly.

Examples include:

- factory construction;
- power-plant construction;
- refinery expansion;
- military procurement;
- shipbuilding;
- aircraft production;
- ammunition production;
- training and mobilization;
- infrastructure repair;
- reconstruction;
- crop planting and harvest;
- herd rebuilding;
- fertilizer carryover effects;
- depleted inventories;
- strategic stockpile exhaustion;
- education cohort progression;
- demographic transitions;
- technology diffusion;
- debt maturity;
- capital depreciation;
- disease incubation;
- environmental degradation and recovery;
- delayed political and fiscal effects.

A core design goal is that disruptions may continue to cause effects after the headline event ends.

Example:

```text
fertilizer disruption ends
    -> inventories are already depleted
    -> planting decisions were altered months earlier
    -> yields decline later
    -> food stocks tighten
    -> food prices rise
    -> real household income falls
    -> fiscal and political effects appear after the original shock
```

This delayed-causality architecture is necessary for realistic system behavior and for the simulation to produce second-order effects that are not simply scripted.

---

# Decision

## 1. Delays are explicit causal mechanisms

The engine will not implement a generic rule such as:

```text
all cross-system effects take one tick
```

or:

```text
every equation can specify arbitrary delay = N
```

without semantic meaning.

A delay must correspond to a modeled process such as:

- construction;
- transport;
- biological growth;
- training;
- institutional adjustment;
- contract maturity;
- production lead time;
- stock depletion;
- information formation;
- expectation adjustment.

---

## 2. Delay mechanisms are classified

Initial classes:

```rust
pub enum DelayMechanism {
    FixedLag,
    Pipeline,
    StockBuffer,
    Queue,
    CohortTransition,
    RollingMemory,
    Diffusion,
    MaturitySchedule,
    RepairProcess,
    ConstructionProcess,
    TrainingProcess,
    BiologicalCycle,
}
```

These are semantic categories, not necessarily separate implementations.

---

## 3. Fixed lags are allowed but should be used sparingly

A fixed lag represents a process where the output is a delayed reference to prior state.

Example:

```text
wage adjustment responds to inflation from 3 months earlier
```

Conceptually:

```rust
pub struct FixedLag {
    pub ticks: u32,
}
```

Use fixed lags only when the delay itself is the intended model.

Do not use them to avoid designing a real stock or pipeline.

---

## 4. Pipelines represent committed processes over time

A pipeline represents something that has started but is not yet complete.

Examples:

- construction;
- procurement;
- training;
- repair;
- crop cycle;
- education progression;
- debt maturity;
- technology adoption.

Conceptual type:

```rust
pub struct PipelineEntry {
    pub id: PipelineId,
    pub owner: EntityId,
    pub pipeline_type: PipelineTypeId,
    pub start_tick: Tick,
    pub expected_completion_tick: Tick,
    pub progress: Fraction,
    pub committed_quantity: Quantity,
    pub committed_resources: ResourceBundle,
    pub status: PipelineStatus,
}
```

---

## 5. Pipeline IDs are stable

```rust
pub struct PipelineId(pub u64);
```

Allocation is deterministic.

Pipelines may be referenced in:

- diagnostics;
- replay;
- causal traces;
- user interface;
- save files.

---

## 6. Pipeline status

```rust
pub enum PipelineStatus {
    Planned,
    Active,
    Paused,
    Delayed,
    Completed,
    Cancelled,
    Failed,
}
```

Status changes are authoritative state transitions.

---

## 7. Pipeline progress is not necessarily linear

Some processes progress evenly.

Others depend on:

- labor;
- capital goods;
- materials;
- energy;
- transport;
- weather;
- damage;
- financing;
- policy;
- enemy action.

Therefore:

```text
progress[t+1] != progress[t] + constant
```

in general.

---

## 8. Completion time may move

A project's expected completion tick may be revised.

Example:

```text
shipyard loses electricity
    -> ship construction slows
    -> delivery moves from month 18 to month 21
```

The engine records the change.

---

## 9. Resources may be committed versus consumed

Pipelines distinguish:

```text
committed resources
consumed resources
remaining resources
```

Example:

A government may authorize a ship purchase before steel, labor, and yard time are fully consumed.

---

## 10. Cancellation has consequences

Cancelling a pipeline may cause:

- sunk cost;
- partial salvage;
- released labor;
- released capacity;
- political effects;
- contract penalties.

Cancellation is not simply deletion.

---

## 11. Delayed-event queue

Some future effects are better represented as scheduled events than continuously tracked pipelines.

Conceptual type:

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

---

## 12. Event queue use cases

Good uses:

- debt maturity;
- treaty expiration;
- scheduled election;
- delivery completion;
- contract expiration;
- sanctions expiration;
- delayed policy activation.

Poor use:

- every monthly stock update;
- continuous construction progress;
- ongoing epidemic state.

---

## 13. Stocks are explicit buffers

Stocks absorb timing mismatch.

Examples:

- fuel inventory;
- food inventory;
- ammunition;
- spare parts;
- strategic petroleum reserve;
- cash reserves;
- hospital capacity buffer;
- fertilizer inventory;
- industrial input stocks.

A stock follows:

```text
stock[t+1]
=
stock[t]
+ inflows
- outflows
- losses
```

subject to capacity and nonnegativity constraints.

---

## 14. Stocks can hide a shock temporarily

This behavior is intentional.

Example:

```text
oil imports fall
but fuel stocks are high
    -> immediate consumption impact small
    -> inventory falls
    -> later shortage appears
```

The visible consequence may therefore lag the original disruption.

---

## 15. Stocks create post-shock effects

A shock can end while stocks remain depleted.

Example:

```text
shipping disruption ends
    -> supply resumes
    -> inventories still low
    -> prices remain elevated
    -> restocking competes with current demand
```

This is a key New Engine simulation principle.

---

## 16. Restocking demand is explicit

When inventories fall below target:

```text
restocking demand
```

may increase after supply resumes.

This can generate temporary overshoot.

Example:

```text
post-crisis imports > normal imports
```

because inventories are rebuilding.

---

## 17. Target stocks may be endogenous

Target inventory may depend on:

- volatility;
- war risk;
- policy;
- storage capacity;
- financing cost;
- expected future prices.

This supports strategic stockpiling behavior.

---

## 18. Buffer depletion is diagnosable

Causal tracing should distinguish:

```text
shock absorbed by buffer
```

from:

```text
shock immediately affected consumption
```

---

## 19. Queues represent unmet work or demand

A queue represents accumulated unresolved demand.

Examples:

- port backlog;
- repair backlog;
- medical waiting list;
- procurement backlog;
- freight backlog;
- housing construction backlog.

Queue dynamics:

```text
queue[t+1]
=
queue[t]
+ arrivals
- processed
- cancellations
```

---

## 20. Queue length can affect system behavior

Examples:

```text
port backlog -> shipping delay
repair backlog -> lower infrastructure availability
medical backlog -> worsened health outcomes
```

---

## 21. Queue aging is optional

Where relevant, queues may track age distribution.

Example:

```text
repair requests older than 6 months
```

This is more expensive and should be fidelity-profile dependent.

---

## 22. Construction is a pipeline, not an instant capacity change

Investment decision:

```text
investment authorized
```

does not immediately become:

```text
productive capacity
```

Instead:

```text
decision
 -> financing/resources
 -> construction pipeline
 -> completion
 -> commissioning
 -> capacity
```

---

## 23. Construction may require commissioning

Some infrastructure requires a final commissioning phase.

Examples:

- nuclear plant;
- refinery;
- military system;
- semiconductor fabrication facility.

Completion and operational availability may differ.

---

## 24. Capital under construction is state

Projects in progress represent:

- sunk capital;
- future capacity;
- material demand;
- labor demand;
- vulnerability to disruption.

They must remain visible to the model.

---

## 25. Repair is separate from construction

Repair pipelines may differ from new construction.

Repair may have:

- shorter lead time;
- different material requirements;
- priority rules;
- emergency capacity.

---

## 26. Reconstruction can generate temporary demand

War or disaster damage can create:

```text
lost capacity
+
reconstruction demand
```

This means some sectors may experience increased demand while the economy as a whole suffers.

---

## 27. Military procurement uses pipelines

Military procurement must separate:

```text
order
production
delivery
training
operational readiness
```

Example:

```text
aircraft ordered
 -> factory slot
 -> production
 -> delivery
 -> crew training
 -> squadron readiness
```

---

## 28. Equipment replacement is not instantaneous

Combat losses reduce equipment immediately.

Replacement depends on:

- existing stocks;
- repair;
- production;
- imports;
- transport;
- training.

---

## 29. Ammunition has both stock and production pipeline

Ammunition behavior:

```text
existing stockpile
+
monthly production
+
imports
-
combat expenditure
-
training expenditure
```

Plant expansion is a separate longer pipeline.

---

## 30. Mobilization uses staged pipelines

Mobilization may include:

```text
notification
assembly
training
equipment assignment
transport
deployment
```

Not all reserve manpower becomes ready immediately.

---

## 31. Training is a time-consuming state process

Training pipelines can apply to:

- military personnel;
- pilots;
- technicians;
- teachers;
- healthcare workers;
- industrial workers.

Education/training improves labor characteristics after elapsed time.

---

## 32. Education is cohort/pipeline based

Education effects should typically flow through:

```text
enrollment
 -> progression
 -> graduation/attainment
 -> workforce entry
 -> productivity/earnings
```

not:

```text
education spending +10%
 -> productivity +X% this month
```

---

## 33. Demographic effects use cohort transitions

Population systems should use cohort/time mechanisms for:

- aging;
- fertility;
- mortality;
- migration integration;
- workforce entry;
- retirement.

---

## 34. Agriculture uses biological cycles

Crop production should separate:

```text
planting decision
input application
growing conditions
harvest
storage
consumption/trade
```

This creates natural delayed response.

---

## 35. Fertilizer shock example

A fertilizer disruption may follow:

```text
Month 1:
    imports collapse

Month 1-3:
    domestic stocks decline
    prices rise

Month 3-5:
    farmers reduce application or acreage

Month 6-10:
    crop develops under lower input

Month 9-12:
    yield declines

Month 10+:
    food inventory tightens
    feed costs rise
    livestock decisions change
    consumer food prices rise
```

The original disruption may have ended before the major food-price effect appears.

---

## 36. Livestock has slower biological pipelines

Livestock systems may model:

- breeding;
- herd expansion;
- slaughter decisions;
- feed cost;
- gestation;
- maturation.

Herd rebuilding can take multiple years.

---

## 37. Technology diffusion is not instant

Technology can move through stages:

```text
available
adoptable
ordered
installed
utilized
diffused
```

Adoption depends on:

- capital;
- skills;
- institutions;
- price incentives;
- trade access;
- policy.

---

## 38. Diffusion model

A generic diffusion process may use:

```text
adoption[t+1]
=
adoption[t]
+
potential_adopters
* adoption_rate
```

with bounded saturation.

Specific technologies may use richer models.

---

## 39. Environmental effects may accumulate

Examples:

- soil degradation;
- groundwater depletion;
- pollution;
- deforestation;
- contamination;
- carbon concentration.

These are typically stocks with slow flows.

---

## 40. Environmental recovery can lag

Stopping pollution does not imply immediate environmental recovery.

Recovery may follow:

```text
pollution source stops
 -> contamination stock remains
 -> gradual decay/remediation
 -> health/ecosystem effect declines later
```

---

## 41. Disease processes may use incubation delays

Health models may include:

```text
exposure
 -> incubation
 -> symptomatic case
 -> hospitalization
 -> recovery/death
```

depending on fidelity profile.

---

## 42. Debt maturity is a schedule

Government/corporate debt should separate:

```text
issuance
outstanding principal
interest payments
maturity/refinancing
```

A rate shock does not instantly reprice all existing fixed-rate debt.

---

## 43. Interest-rate pass-through can be delayed

Example:

```text
new borrowing cost rises now
existing debt reprices gradually as maturities roll
```

This allows realistic fiscal lag.

---

## 44. Expectations are state, not perfect foresight

Actors do not automatically know future model outcomes.

Expectations should be based on:

- recent history;
- trends;
- announced policy;
- observed shocks;
- possibly bounded forecasts.

---

## 45. Expectations types

Initial types:

```rust
pub enum ExpectationModel {
    LastObserved,
    MovingAverage,
    ExponentialSmoothing,
    TrendExtrapolation,
    Anchored,
    PolicyGuided,
}
```

More advanced expectations can be added later.

---

## 46. Exponential smoothing

Example:

```text
expected[t+1]
=
alpha * observed[t]
+
(1 - alpha) * expected[t]
```

This creates persistent but adaptive expectations.

---

## 47. Expectations carry explicit state

Expectation state is authoritative if future decisions depend on it.

Example:

```rust
pub struct PriceExpectationState {
    pub expected_price: PriceIndex,
}
```

---

## 48. Expectations are owned by the deciding subsystem

Example:

```text
Energy investor expectation -> Energy
Household inflation expectation -> Economy
Military threat expectation -> International/Military depending model boundary
```

Ownership must be explicit.

---

## 49. Expectations can be wrong

The simulation should permit:

```text
expected price != actual future price
```

This is desirable.

---

## 50. No hidden perfect foresight

An equation must not read future state unless the model explicitly represents a planning optimization with known future scenario inputs.

---

## 51. Announced policy may affect expectations before implementation

Example:

```text
future tariff announced
 -> firms change inventory/order behavior now
 -> tariff activates later
```

This is modeled through expectations or planned-policy state.

---

## 52. Policy implementation can be delayed

A law may pass now but phase in later.

Example:

```text
tax credit enacted
 -> effective in 6 months
```

Use a scheduled policy event/pipeline.

---

## 53. Administrative capacity can delay execution

Government action may be limited by:

- staffing;
- procurement;
- legal process;
- permitting;
- institutional capacity.

Policy declaration and physical implementation are not always simultaneous.

---

## 54. Sanctions can have staggered effects

Example:

```text
sanction announced
 -> contracts stop
 -> shipments already at sea continue
 -> inventory absorbs shock
 -> replacement suppliers found
 -> long-run trade structure changes
```

This should emerge from trade, inventory, and contract timing.

---

## 55. Shipping has transit time

Trade flows may optionally distinguish:

```text
ordered
in transit
arrived
```

Especially important for:

- oil;
- LNG;
- military equipment;
- grain;
- strategic goods.

---

## 56. Transit pipelines

Conceptually:

```rust
pub struct TransitBatch {
    pub origin: RegionId,
    pub destination: RegionId,
    pub commodity: CommodityId,
    pub quantity: Quantity,
    pub depart_tick: Tick,
    pub arrival_tick: Tick,
}
```

---

## 57. In-transit goods are authoritative stock

Goods in transit are neither at origin nor destination inventory.

They can be:

- delayed;
- rerouted;
- intercepted;
- destroyed.

---

## 58. Lead times should be data-driven where practical

Pipeline duration may depend on:

- asset type;
- country;
- technology;
- capacity;
- wartime conditions;
- infrastructure.

Avoid one universal build time.

---

## 59. Duration distributions

Some processes may use deterministic pseudo-random duration variation.

If so:

- PRNG is keyed/deterministic;
- resulting completion tick is recorded;
- replay remains deterministic.

v0.1 should prefer deterministic duration functions for core infrastructure.

---

## 60. Partial completion effects

Some pipelines produce benefits before full completion.

Example:

```text
road repair opens one lane before full restoration
```

This can be modeled through staged milestones.

---

## 61. Milestones

```rust
pub struct PipelineMilestone {
    pub progress_threshold: Fraction,
    pub effect: MilestoneEffect,
}
```

Milestones are deterministic and ordered.

---

## 62. Pipeline dependencies

Projects may depend on other projects/resources.

Example:

```text
factory expansion waits for:
    grid connection
    imported machinery
    skilled labor
```

Dependencies are explicit.

---

## 63. Pipeline bottleneck rule

Progress can be limited by the scarcest required input.

Conceptually:

```text
progress_rate
=
min(
    labor_factor,
    material_factor,
    energy_factor,
    finance_factor,
    infrastructure_factor
)
```

Specific domains may use more nuanced equations.

---

## 64. No hidden time compression

If a project requires 24 months, the engine should not complete it faster merely because the simulation is running in a faster wall-clock mode.

Simulation speed does not alter model time.

---

## 65. Fast simulation profiles may simplify pipelines

A low-fidelity profile may aggregate stages.

Example:

```text
order -> completion
```

instead of:

```text
order -> fabrication -> transport -> installation -> commissioning
```

But total delay semantics should remain broadly consistent.

---

## 66. Delayed effects and solver iteration are separate

Pipeline progress occurs across committed ticks.

Solver iteration occurs inside one tick.

Do not advance pipeline time during numerical iterations.

---

## 67. One tick means one time advance

Outer solver iterations do not imply repeated passage of a month.

This must be enforced in code.

---

## 68. Pipeline updates occur once per due cadence

Example:

```text
construction progress
```

updates once per monthly tick, not once per solver iteration.

---

## 69. Event activation phase

Per ADR-001, due delayed events activate before the main solve.

Order:

```text
BeginTick
 -> ApplyExogenousInputs
 -> ActivateDueDelayedEvents
 -> PrepareInheritedState
 ...
```

This remains binding.

---

## 70. Pipeline progress phase

Pipeline progress is calculated in the owning subsystem's appropriate phase.

Example:

```text
Energy construction pipeline
    -> Energy solve/update

Military procurement pipeline
    -> StrategicMilitaryPass
```

---

## 71. Pipeline completion publication

Completion updates authoritative owner state and then publishes new contracts at the next defined synchronization barrier.

---

## 72. Same-tick completion behavior

If a project completes during tick `t`, its availability for other subsystems must be explicitly defined.

Initial default:

```text
completion becomes authoritative at commit t
and broadly available through contracts at t+1
```

unless a subsystem-specific barrier explicitly allows same-tick use.

This avoids ambiguous mid-tick visibility.

---

## 73. Emergency same-tick effects

Some events must affect fast systems immediately.

Example:

```text
bridge destroyed during military substep
```

This requires an explicitly defined fast synchronization point.

v0.1 should keep such cases narrow.

---

## 74. Delayed-event causality record

Each event/pipeline should record:

```text
originating shock/action
creation tick
owner
expected completion
actual completion
downstream event IDs
```

where tracing is enabled.

---

## 75. Parent-child causal links

Conceptually:

```rust
pub struct CausalParentId(pub u64);
```

A pipeline can carry:

```text
created_by = shock_123
```

This allows later explanation:

```text
Why did food prices rise now?
Because planting input was reduced 7 months ago after fertilizer shock 123.
```

---

## 76. Historical memory buffers

Some equations require rolling history.

Example:

```text
12-month inflation
3-year rainfall average
5-year fertility trend
```

Use bounded rolling buffers.

---

## 77. Rolling buffer type

Conceptually:

```rust
pub struct RollingSeries<T, const N: usize> {
    values: [T; N],
    cursor: usize,
}
```

Dynamic windows may use compact ring buffers.

---

## 78. Historical memory is subsystem-owned

A subsystem owns history required by its equations.

Do not build one giant global time-series store into authoritative state unless needed.

---

## 79. Long-term analytics are separate

Full historical reporting can be written to:

- telemetry;
- compressed history files;
- analytics database;
- save snapshots.

Not every historical point belongs in hot authoritative memory.

---

## 80. Delayed second-order effect principle

The engine explicitly supports:

```text
shock
 -> immediate physical effect
 -> buffer absorption/depletion
 -> behavioral response
 -> pipeline change
 -> later output consequence
 -> later price/income/fiscal/social response
```

This is a core simulation invariant.

---

## 81. Post-crisis overshoot

The model should permit overshoot after a disruption.

Examples:

- restocking;
- reconstruction;
- delayed purchases;
- replacement orders;
- emergency investment.

---

## 82. Scarring

The model should permit permanent or long-lived effects after a temporary shock.

Examples:

- destroyed capital;
- lost education;
- migration;
- debt accumulation;
- depleted herd;
- supply-chain relocation.

---

## 83. Hysteresis

Some systems may not return to the original state after the shock ends.

This is modeled through state changes, not special-case scripting.

---

## 84. Recovery paths

Recovery may depend on:

- remaining capacity;
- financing;
- labor;
- imports;
- policy;
- security;
- expectations.

Avoid a generic "recovery rate" where a richer causal path is available.

---

## 85. Expectations can affect recovery

Example:

```text
firms expect instability to persist
 -> delay investment
 -> recovery slower
```

---

## 86. Expectations can amplify bubbles

Example:

```text
rising prices
 -> expected future rise
 -> increased demand/investment
 -> further price pressure
```

Such feedback must be bounded and solver-safe.

---

## 87. Expectation update cadence

Expectation models declare cadence.

Examples:

```text
financial expectations -> monthly
capital investment expectations -> quarterly
population expectations -> annual
```

---

## 88. Expectations use only observable/available information

A subsystem should not automatically consume authoritative hidden state if actors would not know it.

Later architecture may distinguish:

```text
authoritative state
perceived state
```

ADR-005 remains compatible with that design.

---

## 89. Delayed policy credibility

Policy credibility may influence expectation adjustment.

Example:

```text
announced inflation target
 -> expectations move partly
```

This belongs to expectation equations, not magic immediate state replacement.

---

## 90. Cargo crate direction

Recommended shared types:

```text
sim-time/
    Tick
    calendar
    lag types

sim-pipelines/
    PipelineId
    PipelineEntry
    delayed event queue
    rolling buffers
    maturity schedules

sim-expectations/
    expectation model traits/helpers
```

These may initially live inside existing crates if separate crates are premature.

---

## 91. Avoid over-fragmenting crates

Do not create a crate for every concept before implementation warrants it.

Initial acceptable placement:

```text
sim-kernel:
    delayed event scheduling

sim-state:
    common pipeline types

subsystem-*:
    domain pipeline state

sim-equations:
    temporal metadata
```

Extract shared crates later if code reuse becomes substantial.

---

## 92. Pipeline trait direction

Conceptual helper:

```rust
pub trait PipelineProcess {
    type State;
    type Inputs;
    type Output;

    fn advance(
        &self,
        state: &mut Self::State,
        inputs: &Self::Inputs,
        ctx: &TickContext,
    ) -> Option<Self::Output>;
}
```

Not every pipeline must implement one generic trait.

---

## 93. Maturity schedules

Useful for:

- debt;
- contracts;
- leases;
- military maintenance cycles.

Conceptually:

```rust
pub struct MaturitySchedule<T> {
    entries: Vec<MaturityEntry<T>>,
}
```

Entries are ordered deterministically.

---

## 94. Duration units

Pipeline duration is expressed in simulation time units.

For monthly profile:

```text
ticks
```

Metadata may also expose human-readable duration.

---

## 95. Calendar-aware processes

Some cycles depend on calendar month.

Example:

- planting season;
- harvest;
- winter energy demand;
- school year.

These should use calendar services, not `tick % N` hacks.

---

## 96. Seasonal pipelines

Agriculture may use region-specific calendars.

Example:

```text
corn planting:
    April-May
harvest:
    September-October
```

Data-driven calendars are preferable.

---

## 97. Seasonal shock timing matters

A fertilizer disruption in January and one in May may have very different agricultural consequences.

The engine must preserve that distinction.

---

## 98. Lead-time shock timing matters

A shipyard strike near delivery may differ from one early in construction.

Progress state makes this visible.

---

## 99. Buffers have capacity

Stocks may have maximum storage.

Example:

```text
fuel tank capacity
grain silo capacity
strategic reserve capacity
```

Restocking cannot exceed physical capacity.

---

## 100. Buffer losses

Some stocks decay.

Examples:

- food spoilage;
- fuel loss;
- medical supplies expiration;
- ammunition degradation.

Use explicit loss flows.

---

## 101. Buffer priority rules

Scarce stock may be allocated by priority.

Example:

```text
fuel:
    military
    emergency services
    food transport
    general industry
```

Allocation belongs to the owning subsystem's model.

---

## 102. Rationing

Rationing is a policy/allocation rule that changes how stock depletion affects consumers.

It does not create supply.

---

## 103. Queue priority

Queues may use deterministic priority classes.

Example:

```text
critical repair
military repair
commercial repair
routine repair
```

Tie-break by stable ID.

---

## 104. Backlog can create political pressure

Backlog variables may publish contracts to Governance/Human Development.

Example:

```text
hospital waiting backlog
 -> satisfaction/health consequences
```

---

## 105. Delayed fiscal effects

Policy cost may unfold over time.

Example:

```text
subsidy authorized now
 -> claims filed later
 -> government outlays later
```

Use payment schedules where appropriate.

---

## 106. Tax lags

Tax revenue may follow economic activity with collection delay.

Example:

```text
corporate profit this quarter
 -> tax payment next quarter
```

This should be modeled explicitly where important.

---

## 107. Insurance/claims delays

Disaster loss may produce later fiscal/financial outlays.

This can be a delayed event/queue.

---

## 108. Reconstruction financing lag

Damage today may not generate full reconstruction spending until financing and contracts are arranged.

---

## 109. Trade substitution lag

Replacing a supplier takes time.

Possible mechanism:

```text
lost supplier
 -> search/contracting
 -> transport setup
 -> new flow
```

Not all imports should instantly reroute.

---

## 110. Industrial retooling lag

A factory switching production may require:

- tooling;
- training;
- certification;
- inputs.

Use pipeline state.

---

## 111. Wartime production ramp

Example:

```text
mobilization policy
 -> orders
 -> overtime
 -> existing capacity utilization rises quickly
 -> new tooling/capacity arrives later
```

Different responses occur at different delays.

---

## 112. Capacity utilization vs capacity expansion

Short-run response:

```text
utilization increases
```

Long-run response:

```text
new capacity built
```

These must be separate variables.

---

## 113. Maintenance backlog

High utilization can create maintenance backlog.

Later:

```text
availability falls
```

This creates delayed operational cost.

---

## 114. Wear accumulation

Equipment/infrastructure may accumulate wear as a stock.

Repair/maintenance reduces wear.

---

## 115. Deferred maintenance

Budget cuts may not cause immediate collapse.

They may produce:

```text
maintenance backlog
 -> wear
 -> reliability decline later
```

This is a useful delayed-causality mechanism.

---

## 116. Human capital scarring

Education disruption can produce long-delayed labor effects.

Example:

```text
school closures
 -> lower attainment
 -> workforce skill effect years later
```

---

## 117. Migration pipeline

Migration may have:

```text
decision
departure
transit
arrival
integration
```

Fidelity dependent.

---

## 118. Refugee effects

Conflict may generate:

```text
displacement immediately
 -> host-region population pressure
 -> labor-market integration later
```

---

## 119. Political effects can lag

Economic pain may affect:

- approval;
- protest;
- electoral behavior;
- policy response;

with configurable lag/memory.

Do not assume immediate one-for-one political response.

---

## 120. Memory decay

Some social/political effects use decaying memory.

Example:

```text
perceived recent inflation
```

can use exponential smoothing.

---

## 121. Event expiration

Temporary effects need explicit end conditions.

Examples:

- emergency powers;
- temporary sanctions;
- disaster state;
- rationing.

Expiration can be scheduled or state-dependent.

---

## 122. Conditional completion

Not every pipeline completes at a fixed date.

Example:

```text
construction completes when progress >= 1.0
```

Expected completion is forecast, not authority.

---

## 123. Paused pipelines

Projects may pause due to:

- financing;
- material shortage;
- war;
- regulation;
- damage.

Paused pipelines retain state.

---

## 124. Pipeline failure

A pipeline may fail permanently.

Example:

- bankruptcy;
- destruction;
- canceled treaty;
- destroyed construction site.

Failure consequences are domain-specific.

---

## 125. Pipeline salvage

Partial value may remain after failure/cancellation.

This may return:

- materials;
- equipment;
- capital value.

---

## 126. Delayed event cancellation

Scheduled events need cancellation handles where appropriate.

Example:

```text
planned sanctions activation canceled before due date
```

Cancellation must be deterministic and logged.

---

## 127. Event idempotence

A delayed event must not apply twice.

The queue should enforce single activation semantics.

---

## 128. Save/replay compatibility

Save files must include:

- active pipelines;
- progress;
- committed resources;
- delayed events;
- rolling memory state;
- expectation state;
- maturity schedules;
- queue backlogs.

Otherwise replay after load would diverge.

---

## 129. State hashing

Authoritative delayed-state structures participate in state hashing.

Canonical order is required.

---

## 130. Deterministic pipeline iteration

Canonical pipeline order:

```text
pipeline_type
owner_id
pipeline_id
```

or another explicitly defined stable order.

---

## 131. Deterministic event ordering

ADR-001 ordering remains binding:

```text
due_tick
subsystem_id
entity_id
sequence
```

---

## 132. Pipeline diagnostics

Developer tools should report:

```text
active count
paused count
average delay
completion slippage
resource bottleneck
```

---

## 133. Shock-to-delay tracing

Causal tools should support:

```text
shock
 -> pipeline
 -> completion/change
 -> downstream variable
```

---

## 134. Example trace

```text
FoodPriceIndex rose in 2028-10

because:
    GrainInventory fell
because:
    WheatYield fell in 2028-09
because:
    FertilizerApplication fell during planting
because:
    FertilizerImports were disrupted in 2028-02
```

This is a target diagnostic capability.

---

## 135. Pipeline graph

Developer tooling should eventually export:

```text
project type
dependencies
resources
lead time
downstream state
```

---

## 136. Time-to-effect metadata

Equation or pipeline metadata may expose typical time-to-effect ranges for diagnostics/documentation.

This is descriptive, not necessarily a runtime rule.

---

## 137. Scenario authoring

Scenario authors should preferably modify:

- initial stocks;
- capacity;
- pipeline state;
- event schedule;
- policy;
- imports;
- physical damage;
- expectations inputs.

Avoid authoring downstream outcomes directly.

---

## 138. Historical scenarios

Historical initialization may need existing in-progress pipelines.

Example:

```text
ships already under construction
projects already funded
debt already maturing
```

Initial state loader must support that.

---

## 139. Fictional scenarios

The same mechanisms apply to fictional worlds.

Pipeline types and durations come from profile data.

---

## 140. Fidelity scaling

High fidelity:

```text
multi-stage construction
transport pipeline
commissioning
```

Low fidelity:

```text
single aggregate pipeline
```

Contract semantics should remain stable.

---

## 141. Performance strategy

Avoid one heap object per trivial delayed effect if scale becomes large.

Potential optimizations:

- grouped pipeline arrays;
- bucketed due-event queues;
- structure-of-arrays storage;
- per-subsystem timing wheels;
- compact ring buffers.

Optimization must preserve canonical semantics.

---

## 142. Timing wheel option

A future high-performance event queue may use a deterministic timing wheel.

v0.1 may use ordered vectors/heaps if performance is sufficient.

---

## 143. Priority queue requirement

If using `BinaryHeap`, remember Rust's default is max-heap.

Wrap ordering explicitly.

Never rely on pointer/insertion accident.

---

## 144. Cargo tests

Required delayed-causality tests:

```text
pipeline completion
pipeline pause/resume
event ordering
event cancellation
stock depletion
restocking overshoot
fixed-lag correctness
rolling-memory correctness
save/load replay
```

---

## 145. Fertilizer regression test

A mandatory architecture test should simulate:

```text
temporary fertilizer import disruption
```

and verify:

- no immediate scripted food output collapse;
- fertilizer stocks absorb some shock;
- planting/input decisions change;
- crop yield effect occurs later;
- food inventory/price effect occurs after harvest;
- recovery may continue after imports resume.

---

## 146. Military procurement regression test

Scenario:

```text
major equipment losses
```

Verify:

- immediate equipment count declines;
- reserve stock can replace some losses;
- new procurement creates pipeline;
- delivery occurs later;
- readiness recovery lags delivery if training required.

---

## 147. Infrastructure damage regression test

Scenario:

```text
power plant damaged
```

Verify:

- capacity falls immediately;
- repair pipeline starts;
- repair consumes resources;
- capacity returns only on progress/completion;
- downstream economic effects respond through contracts.

---

## 148. Debt maturity regression test

Scenario:

```text
interest rates rise
```

Verify:

- newly issued debt cost rises quickly;
- existing fixed debt does not all reprice instantly;
- fiscal burden rises as maturity/refinancing occurs.

---

## 149. Expectation regression test

Scenario:

```text
temporary price spike
```

Verify:

- expected price responds according to configured model;
- expectations decay/adapt;
- expectations do not read future true prices.

---

## 150. CI validation

`sim-tools` should eventually support:

```powershell
cargo run -p sim-tools -- pipelines validate
```

Checks:

- invalid negative durations;
- missing owners;
- unknown pipeline types;
- impossible resource units;
- unordered milestones;
- duplicate event IDs.

---

## 151. Model review questions

Any new delayed mechanism should answer:

```text
What real process causes the delay?
What state persists while waiting?
Can the process pause?
Can it fail?
What resources does it consume?
What determines completion?
What does completion change?
Can the effect be partially realized?
```

---

## 152. Rejected alternatives

### Generic one-tick lag on all cross-system effects

Rejected because real time constants vary dramatically.

### Direct future outcome scripting

Rejected because it breaks causal emergence.

### Instant capacity from investment

Rejected because construction takes time.

### Perfect foresight expectations

Rejected as the default because actors should not know future simulation state.

### Treating stocks as diagnostics only

Rejected because inventories and reserves are causal buffers.

### Advancing pipelines during solver iterations

Rejected because numerical iterations do not represent passage of time.

---

## 153. Consequences

### Positive

- delayed second-order effects emerge naturally;
- temporary shocks can have persistent consequences;
- buffers and inventories matter;
- construction/procurement become realistic;
- agricultural timing matters;
- post-crisis overshoot becomes possible;
- policy and debt effects can lag;
- causal explanation improves;
- solver iteration is not abused to simulate time.

### Costs

- more state to save/hash;
- more pipeline bookkeeping;
- additional calibration of lead times;
- more complex scenario initialization;
- some UI/tooling needed to inspect hidden future processes;
- high-fidelity logistics/transit can become expensive.

These costs are accepted.

---

## 154. Open implementation questions

Still open:

- exact generic pipeline storage;
- timing wheel vs ordered queue;
- facility ownership between Infrastructure/Energy;
- trade transit granularity;
- exact expectation models per subsystem;
- whether rolling history uses generic ring-buffer helpers;
- event cancellation-handle design;
- milestone API;
- whether high-volume queues use age cohorts or aggregate backlog.

These do not change the core decision.

---

## 155. Acceptance criteria

ADR-005 is implemented when:

1. delayed-event queue exists;
2. deterministic event ordering exists;
3. generic pipeline state exists;
4. pipeline pause/resume exists;
5. stock-buffer mechanism exists;
6. rolling-memory helper exists;
7. at least one expectation model exists;
8. fertilizer delayed-shock test passes;
9. military procurement delayed-delivery test passes;
10. save/replay preserves delayed state;
11. delayed state participates in hashing;
12. no pipeline advances during solver iteration;
13. causal tracing can link a later result to an earlier shock/pipeline.

---

# Decision summary

New Engine will model delayed consequences through explicit causal state:

- pipelines;
- delayed events;
- inventories and reserves;
- queues and backlogs;
- cohort transitions;
- rolling memory;
- maturity schedules;
- repair/construction processes;
- biological cycles;
- expectation state.

A disruption can end before its most important downstream consequences appear.

Stocks may temporarily hide shocks, depleted buffers may prolong them, pipelines may shift effects into the future, and expectations may alter recovery.

Numerical solver iteration does not advance time.

This ADR establishes the temporal-causality foundation for New Engine's long-horizon world simulation.
