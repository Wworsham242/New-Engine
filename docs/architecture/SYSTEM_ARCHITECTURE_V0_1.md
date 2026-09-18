# New Engine — System Architecture v0.1

**Status:** Architecture baseline  
**Repository target:** `Wworsham242/New-Engine`  
**Primary implementation language:** C++  
**Document purpose:** Define the executable simulation architecture for a deterministic, high-fidelity grand-strategy / geopolitical world engine.

## 1. Purpose

This document defines the first serious architecture baseline for New Engine.

The engine simulates a world as an interacting system of systems rather than as disconnected game mechanics. Economic activity, energy, agriculture, government, infrastructure, population, health, education, environment, international relations, and military activity influence one another through explicit causal interfaces.

The architecture must support deterministic execution and replay, reproducible scenario experiments, multiple simulation cadences, local feedback loops and iterative solvers, delayed and second-order consequences, stocks/flows/buffers/queues/pipelines, explicit subsystem ownership of state, structured cross-subsystem contracts, scalable fidelity, historical and fictional scenarios, causal diagnostics, and integrated military/civilian effects.

A scenario should modify primitive causes whenever possible. Example: a refinery strike reduces refinery capacity, fuel availability, or throughput. It does not directly apply `GDP -5%`. Downstream outcomes emerge through the causal network.

## 2. Core Design Principles

### 2.1 Cause before outcome

Player actions and scenario inputs should change physical capacity, resource availability, policy parameters, access, technology, infrastructure, force posture, or other causal state before touching derived outcomes.

### 2.2 State is owned

Every authoritative variable has one owning subsystem. Other subsystems consume published interface values rather than arbitrary internal state.

### 2.3 Feedback is explicit

The dependency graph is allowed to contain cycles. Feedback may close inside one substep, inside a master tick, across several ticks, or through multi-year pipelines.

### 2.4 Time constants matter

The engine distinguishes fast state, slow structural state, and lagged/pipeline state.

### 2.5 Determinism is mandatory

The same initial state, configuration, actions, scenario inputs, data version, and deterministic random streams must produce the same authoritative result.

## 3. Reference Models and Provenance

### 3.1 International Futures (IFs)

IFs is a design-reference and validation-reference source.

Controlled local analysis of IFs informed several architectural principles:

- recognizable subsystem/module boundaries;
- dense local and cross-system feedback;
- strong Economy↔Governance, Economy↔Energy, Economy↔Agriculture, and demographic interfaces;
- bridge variables that carry information between subsystems;
- same-period and delayed causal effects;
- scenario multipliers that trigger endogenous downstream responses.

IFs is **not** a runtime dependency, code source, equation-copying target, or proprietary implementation to reproduce.

New Engine uses original implementation, original data structures, and original equations or equations independently derived from public literature.

### 3.2 Provenance tags

Major model components should carry provenance such as:

- `ORIGINAL_DESIGN`
- `IFS_OBSERVED_ARCHITECTURE`
- `PUBLIC_LITERATURE`
- `DATASET_DERIVED`
- `MILITARY_REFERENCE_MODEL`
- `PERFORMANCE_SIMPLIFICATION`
- `GAMEPLAY_ABSTRACTION`

## 4. High-Level Architecture

```text
+--------------------------------------------------------------+
|                       Simulation Kernel                      |
| Clock / Scheduler / Determinism / Replay / Diagnostics      |
| Dependency Graph / Contract Bus / State Commit / Jobs       |
+--------------------------------------------------------------+
            |                |                |
            v                v                v
+----------------+  +----------------+  +----------------------+
| Civilian Core  |  | Strategic Core |  | Environment/Physical |
+----------------+  +----------------+  +----------------------+
| Demographics   |  | Governance     |  | Land                 |
| Economy        |  | International  |  | Water                |
| Energy         |  | Military       |  | Climate              |
| Agriculture    |  | Security       |  | Pollution            |
| Infrastructure |  | Diplomacy      |  | Resources            |
| Education      |  +----------------+  +----------------------+
| Health         |
| Human Dev.     |
+----------------+
```

Subsystems communicate through typed contracts. Internal state remains owned by its subsystem.

## 5. Simulation Kernel

The kernel is domain-agnostic.

It owns:

- master simulation time;
- calendar conversion;
- scheduler;
- subsystem registration;
- dependency ordering;
- iteration groups;
- typed interface exchange;
- delayed-event queues;
- deterministic job dispatch;
- state snapshots and commits;
- replay logs;
- checkpointing;
- validation;
- diagnostics;
- performance instrumentation.

The kernel does not determine GDP, energy production, food demand, fertility, readiness, health outcomes, or political outcomes.

## 6. Temporal Model

### 6.1 Master clock

The engine has one authoritative discrete master clock. A master tick advances the authoritative world state by the configured base interval.

The modern-world profile should support a practical monthly or quarterly base cadence, while selected subsystems may substep internally.

### 6.2 Cadence classes

| Cadence | Typical uses |
|---|---|
| Daily/weekly substep | combat, attrition, logistics, acute shortages, crisis operations |
| Monthly | inventories, prices, energy balancing, trade adjustment, readiness |
| Quarterly | GDP components, employment, investment, budgets, procurement |
| Annual | demographic cohorts, education attainment, structural productivity |
| Multi-year | major capital projects, doctrine shifts, technology diffusion |

These are defaults, not hard-coded rules.

### 6.3 Substeps

A subsystem may run deterministic internal substeps without advancing global time.

Example:

```text
Quarterly master tick
    Military:      13 weekly substeps
    Energy:         3 monthly substeps
    Economy:        1 quarterly solve
    Demographics:   accumulate flows; annual cohort transition at year boundary
```

### 6.4 Tick phases

```text
0. BeginTick
1. ApplyExogenousInputs
2. ActivateDuePipelinesAndDelayedEvents
3. PrepareInheritedState
4. FastPhysicalAndOperationalPass
5. CoreCoupledSolve
6. SocialStructuralPass
7. StrategicMilitaryPass
8. EnvironmentResourcePass
9. ReconciliationAndConstraintPass
10. QueueFutureEffects
11. CommitState
12. EmitDiagnosticsAndReplayRecord
13. EndTick
```

The exact sequence can evolve through ADRs.

### 6.5 Core coupled solve

The initial tightly coupled set is expected to include Economy, Energy, Agriculture, Governance/Fiscal, and selected Infrastructure calculations.

```text
for iteration in 0..MAX_CORE_ITERATIONS:
    economy.solve_iteration(inputs)
    energy.solve_iteration(inputs)
    agriculture.solve_iteration(inputs)
    governance.solve_iteration(inputs)
    infrastructure.solve_iteration(inputs)

    publish_contracts()
    residual = measure_core_residual()

    if residual <= tolerance:
        break
```

Order, damping, tolerance, and iteration cap are deterministic and recorded in replay metadata.

### 6.6 Non-convergence

Non-convergence must be handled deterministically:

1. run the configured iterations;
2. apply deterministic damping if enabled;
3. stop at the fixed cap;
4. use a defined fallback result;
5. emit a diagnostic;
6. optionally fail in strict-validation mode.

## 7. State Time Semantics

### 7.1 Fast state

Examples: prices, shortages, inventories, fuel availability, trade flows, readiness, utilization.

### 7.2 Slow state

Examples: population cohorts, productive capital, refineries, power plants, roads, ports, schools, hospitals, military platforms, industrial tooling.

### 7.3 Pipeline state

Examples: construction, procurement, training, crop cycles, repairs, reconstruction, technology adoption, education cohorts, debt maturity.

```cpp
struct PipelineEntry {
    EntityId owner;
    PipelineType type;
    Tick start_tick;
    Tick completion_tick;
    Quantity committed_quantity;
    ResourceBundle committed_resources;
    PipelineStatus status;
};
```

### 7.4 Historical reads

Equations may explicitly request current state, previous committed state, moving averages, rolling windows, trends, or baseline comparisons.

## 8. Equation Architecture

Equations are first-class model components.

### 8.1 Descriptor

```cpp
struct EquationDescriptor {
    EquationId id;
    SubsystemId owner;
    VariableId output;
    EquationClass equation_class;
    Cadence cadence;

    std::span<const VariableId> local_inputs;
    std::span<const ContractFieldId> external_inputs;

    bool uses_previous_state;
    bool iterative;

    double convergence_tolerance;
    uint32_t max_iterations;

    ProvenanceTag provenance;
};
```

Hot-path code can remain compiled C++; descriptors exist for scheduling, validation, diagnostics, and tooling.

### 8.2 Equation classes

The architecture recognizes:

- accounting identities;
- stock-flow equations;
- behavioral equations;
- capacity equations;
- allocation equations;
- market-clearing/adjustment equations;
- empirical equations;
- lag equations;
- transition equations;
- constraint equations.

Examples:

```text
stock(t+1) = stock(t) + inflow - outflow
```

```text
actual_output <= installed_capacity * availability * utilization
```

### 8.3 Ownership

Each equation belongs to one subsystem. Cross-system inputs arrive through typed contracts.

### 8.4 Purity where practical

Prefer:

```text
inputs + previous state + parameters -> result
```

Side effects should occur in application/commit phases rather than deep inside equation code.

## 9. Variable Model

```cpp
struct VariableMetadata {
    VariableId id;
    SubsystemId owner;
    Unit unit;
    ValueKind value_kind;
    Cadence cadence;
    AggregationRule aggregation;
    TemporalSemantics temporal_semantics;
    Bounds bounds;
    ProvenanceTag provenance;
};
```

The model distinguishes stocks, flows, rates, indexes, and parameters and records dimensional units and scope.

## 10. Subsystem Contracts

Subsystem boundaries are API boundaries.

Contracts are typed, versioned, immutable during a solve phase, explicit about units and timing, and traceable in diagnostics.

There should not be a global mutable state bag that every subsystem can modify.

Illustrative contracts:

```cpp
struct DemographicsOutputs {
    Population total_population;
    Population working_age_population;
    Population youth_population;
    Population elderly_population;
    PopulationGrowthRate growth_rate;
    HouseholdCount households;
    MigrationFlows migration;
};

struct EnergyToEconomy {
    EnergyQuantity production;
    EnergyQuantity demand;
    EnergyQuantity imports;
    EnergyQuantity exports;
    EnergyQuantity stocks;
    EnergyPriceIndex price_index;
    ShortageIndex shortage;
    CapacityUtilization utilization;
    EnergyIntensity energy_intensity;
};

struct EconomyToEnergy {
    RealGDP gdp;
    GDPPerCapita gdp_per_capita;
    SectorDemand industrial_activity;
    InvestmentDemand investment;
    ImportCapacity import_capacity;
    ExportDemand export_demand;
    ExchangeRateState exchange_rate;
};
```

## 11. Core Civilian Subsystems

### 11.1 Demographics

Owns population, cohorts, births, demographic deaths, migration stocks/flows, and household counts where appropriate.

### 11.2 Economy

Owns sector production, value added, GDP, consumption, savings, investment, labor demand, wages, prices, imports/exports, trade balances, and configured financial flows.

### 11.3 Energy

Owns energy production by carrier, reserves/resources, conversion, demand, stocks, imports/exports, shortages, investment, capacity, utilization, and prices.

Physical quantities remain distinct from monetary values.

### 11.4 Agriculture and Food

Owns land, yields, crop/livestock production, inputs, demand, inventories, imports/exports, food prices, production losses, and planting/harvest pipelines.

### 11.5 Governance and Fiscal

Owns tax policy, tax revenue, spending, transfers, public investment, debt, debt service, fiscal balance, institutional capacity, and selected governance/security indicators.

### 11.6 Infrastructure

Owns physical stocks and service levels for electricity, transport, ports, roads/rail, water, sanitation, communications/ICT, maintenance, construction, damage, and repair.

### 11.7 Education

Owns enrollment, progression, attainment, education capacity, spending allocation, and skill/human-capital outputs.

### 11.8 Health

Owns morbidity, mortality drivers beyond basic demographic accounting, health capacity, disease dynamics where modeled, nutrition-health effects, and health expenditure requirements.

### 11.9 Environment and Resources

Owns land quality, water resources, pollution, climate state used by game systems, environmental damage, and natural-resource constraints.

### 11.10 Human Development

Owns derived welfare/deprivation outputs such as poverty and composite human-development indicators. It consumes economy/health/education/demographic outputs rather than duplicating them.

### 11.11 International Relations

Owns diplomacy, alliances, sanctions state, international institutions where modeled, strategic threat relationships, foreign aid, access, and international power indicators.

## 12. Military and Warfare

Military simulation is integrated with the civilian world.

### 12.1 Military owns

- force structure;
- unit/equipment stocks;
- readiness;
- training;
- personnel;
- mobilization;
- deployment;
- logistics;
- fuel;
- ammunition;
- maintenance;
- attrition;
- procurement;
- replacement pipelines;
- operational posture;
- combat outcomes;
- military-caused infrastructure damage.

### 12.2 Military consumes

From Economy: fiscal capacity, industrial output, import capacity, labor opportunity cost, prices, investment capacity.

From Energy: fuel availability, fuel prices, electricity, refinery/product constraints.

From Demographics: eligible manpower and cohort sizes.

From Infrastructure: ports, rail, roads, airfields, power, communications, repair capacity.

From Governance: budget, mobilization law, procurement policy, political constraints, institutional capacity.

From International Relations: alliances, access, basing, sanctions, external threat.

### 12.3 Military publishes

- personnel demand;
- casualties;
- equipment losses;
- procurement demand;
- ammunition demand;
- fuel demand;
- transport demand;
- industrial demand;
- fiscal burden;
- infrastructure damage;
- territorial/security state;
- trade disruption;
- mobilization effects;
- reconstruction requirements.

### 12.4 Combat cadence

Combat may resolve daily/weekly internally while civilian consequences are published at synchronization points.

## 13. Shock and Scenario Architecture

A shock changes causal state, policy, capacity, access, or parameters.

```cpp
struct Shock {
    ShockId id;
    Tick effective_tick;
    Tick optional_end_tick;
    Scope scope;
    ShockTarget target;
    ShockOperation operation;
    double magnitude;
    ProvenanceTag provenance;
};
```

Good scenario inputs:

- refinery capacity -30%;
- port throughput -50%;
- fertilizer imports unavailable;
- tariff +10 percentage points;
- reserve mobilization activated;
- bridge destroyed;
- drought yield modifier;
- export ban.

Avoid direct derived-outcome scripting such as `GDP *= 0.95` unless GDP itself is intentionally being used as an exogenous scenario input.

## 14. Delayed Effects and Causal Pipelines

Delayed second-order effects are a required feature.

Example:

```text
fertilizer disruption
 -> higher fertilizer price / lower availability
 -> changed planting/input decisions
 -> months later lower crop yield
 -> lower food inventories
 -> higher food prices
 -> lower household real income
 -> nutrition/fiscal/political effects
```

The original disruption can end before the largest consequence appears.

Delay is modeled through stocks, queues, pipelines, moving averages, expected values, and depleted buffers. There is no generic “delay every edge one tick” rule.

## 15. Solver Architecture

### 15.1 Local feedback solvers

Energy:

```text
production <-> demand <-> imports/exports <-> stocks
<-> shortages <-> utilization <-> price <-> investment
```

Agriculture:

```text
production/yield/land/input
    <->
demand/market/stock/trade
```

Economy:

```text
production <-> income <-> consumption <-> investment
<-> labor <-> prices <-> trade <-> savings
```

### 15.2 Solver policy

```cpp
struct SolverPolicy {
    double absolute_tolerance;
    double relative_tolerance;
    double damping;
    uint32_t max_iterations;
    NonConvergencePolicy failure_policy;
};
```

### 15.3 Outer coupling

Local convergence does not imply global equilibrium. A bounded outer loop may repeat tightly coupled subsystems. The target is stable, causal, deterministic behavior at the intended fidelity, not arbitrary-cost perfect equilibrium.

## 16. Constraint and Reconciliation Pass

After calculations, enforce named physical/accounting constraints such as:

- nonnegative stocks;
- population consistency;
- budget identities;
- import/export availability;
- capacity limits;
- inventory conservation;
- equipment conservation;
- energy balance;
- land-use limits.

Corrections must be attributable and diagnostic rather than hidden.

## 17. Deterministic Parallelism

Independent jobs may run in parallel only when authoritative results remain deterministic.

Requirements:

- fixed dependency graph;
- stable reductions;
- fixed iteration order;
- versioned PRNG if stochastic systems are enabled;
- recorded seeds;
- random draw order independent of thread timing.

## 18. Replay, Saves, and Controlled Experiments

A run must be reproducible from:

```text
Engine version
Data version
Scenario definition
Initial snapshot
Simulation profile
Solver policies
Seed set
Player/AI actions
External event stream
```

The engine must directly support baseline-vs-shock experiments:

```text
baseline run
vs.
shock run

difference(state_baseline, state_shock)
```

## 19. Causality and Diagnostics

The engine should optionally answer “why did this change?”

Example:

```text
FoodPrice +8.4%
 <- DomesticFoodProduction -5.2%
    <- Yield -4.1%
       <- FertilizerAvailability -22%
 <- FoodStocks -11.7%
 <- ImportPrice +3.8%
```

Diagnostic modes should include dependency inspection, causal traces, baseline-vs-scenario decomposition, convergence logs, contract traffic, constraint corrections, and delayed-event provenance.

## 20. Fidelity Profiles

Different worlds and eras should not pay for irrelevant complexity.

Profiles may vary:

- active subsystems;
- sector count;
- commodity count;
- demographic resolution;
- geography;
- military detail;
- cadence;
- equation variants;
- trade-network fidelity;
- environmental resolution.

Subsystem interfaces remain stable while implementations scale.

## 21. Geography and Scope

Variables may exist at:

- world;
- market region;
- country;
- administrative region;
- province;
- grid/raster cell;
- infrastructure node;
- military formation;
- aggregate producer;
- demographic cohort.

Scope is part of variable metadata. Aggregation/disaggregation must be explicit.

## 22. Data Layer

Separate:

1. authoritative simulation state;
2. calibration/reference data;
3. scenario data;
4. configuration;
5. presentation/localization.

Reference data is immutable during a run. Hot-path state should use compact stable IDs and contiguous storage where practical.

## 23. C++ Structural Direction

```cpp
class ISubsystem {
public:
    virtual ~ISubsystem() = default;

    virtual SubsystemId id() const = 0;
    virtual void begin_tick(const TickContext&) = 0;
    virtual void solve(const SolveContext&) = 0;
    virtual void publish(ContractWriter&) const = 0;
    virtual void reconcile(const ReconcileContext&) = 0;
    virtual void commit(const CommitContext&) = 0;
};

class SimulationKernel {
public:
    void step();

private:
    SimulationClock clock_;
    Scheduler scheduler_;
    ContractBus contracts_;
    DelayedEventQueue delayed_;
    SnapshotManager snapshots_;
    ReplayRecorder replay_;
    Diagnostics diagnostics_;
};
```

High-level polymorphism is acceptable; hot per-entity loops should prefer static/contiguous data-oriented code.

## 24. Validation Strategy

### Unit tests
Test equations and constraints.

### Property tests
Examples: no negative physical stocks; equipment conservation; exports respect availability; population identities hold.

### Golden tests
Known scenarios produce stable expected outputs.

### Shock-response tests
Examples: oil-production shock, port closure, fertilizer shock, mobilization, infrastructure destruction, fiscal shock.

### Determinism tests
Repeat identical scenarios and compare authoritative state hashes.

## 25. IFs Refer-Back Policy

Relevant design records should include:

```text
Reference observation:
    What behavior or architecture was observed in IFs?

Adopted principle:
    What general architectural idea was taken from the observation?

New Engine implementation:
    What original design implements the principle?

Validation:
    What test demonstrates the desired behavior?
```

Example:

```text
Reference observation:
    A controlled energy-production shock propagated through energy,
    macroeconomic, fiscal, agricultural, and international variables.

Adopted principle:
    Energy shocks enter through physical energy state and propagate via
    explicit subsystem interfaces.

New Engine implementation:
    Energy capacity/production shock -> Energy solver -> typed contracts.

Validation:
    EnergyShockPropagation golden experiment.
```

Do not store copied proprietary IFs source code in the repository. Do not require IFs to build or run New Engine.

## 26. Architecture Decision Records

Use:

```text
docs/architecture/adr/
```

Initial ADRs:

- ADR-001: Deterministic discrete-time simulation kernel
- ADR-002: Subsystem state ownership
- ADR-003: Typed cross-subsystem contracts
- ADR-004: Multi-cadence scheduling and substeps
- ADR-005: Local iterative solvers plus bounded outer coupling
- ADR-006: Delayed-effect pipelines
- ADR-007: Military as integrated world subsystem
- ADR-008: Reference-model provenance policy

## 27. Initial Implementation Sequence

### Phase A — Kernel skeleton

Implement clock, phases, subsystem registry, typed contracts, snapshots/commit, replay metadata, and diagnostics hooks.

### Phase B — Minimal vertical causal slice

Implement:

```text
Demographics
    |
Economy <-> Energy
    |
Governance
```

Add one controlled energy shock.

Goal: a primitive energy shock produces deterministic, explainable economic and fiscal consequences without scripted GDP effects.

### Phase C — Agriculture and infrastructure

Add agriculture, food stocks, land/yield, infrastructure, and construction/repair pipelines.

### Phase D — Military vertical slice

Add force stock, fuel, readiness, ammunition, procurement, losses, and infrastructure damage.

Goal:

```text
military action
 -> physical loss/damage
 -> energy/infrastructure/economy consequences
 -> fiscal/industrial response
 -> replenishment constraints
```

### Phase E — Social systems

Add education, health, human development, and richer demographics.

### Phase F — International/environmental expansion

Add diplomacy, sanctions, aid, environmental/resource feedback, and advanced trade networks.

## 28. Architectural Invariants

1. Simulation execution is deterministic.
2. State has explicit ownership.
3. Cross-subsystem dependencies use declared contracts.
4. Time semantics are explicit.
5. Equations have owners and provenance.
6. Stocks and flows are distinct.
7. Delays are causal, not arbitrary tick offsets.
8. Subsystems may iterate locally.
9. Global feedback does not imply one giant global solver.
10. Scenario inputs should act on causes rather than script downstream results.
11. Military activity participates in the same civilian causal world.
12. Fidelity is configurable without changing fundamental interfaces.
13. IFs is a reference/validation source, never a runtime dependency or code source.
14. The engine must be able to replay and explain its own outcomes.

## 29. Open Questions

Still unresolved by design:

- default modern-world master tick: monthly vs quarterly;
- exact outer-loop coupling order;
- subsystem convergence metrics;
- monetary/financial depth;
- bilateral vs pooled world markets by commodity;
- geographic granularity;
- military operational time resolution;
- expectations/forecast modeling;
- cross-platform floating-point guarantees;
- compiled vs data-driven equation registration;
- runtime model hot-swapping.

These should be resolved through prototypes and ADRs.

## 30. Definition of Architecture v0.1 Success

Architecture v0.1 is validated when the project can implement a minimal deterministic experiment with:

1. explicit initial state;
2. Demographics, Economy, Energy, and Governance subsystems;
3. typed contracts;
4. at least one local feedback solver;
5. at least one delayed pipeline;
6. one primitive energy shock;
7. deterministic repeatability;
8. baseline-vs-shock comparison;
9. causal diagnostics showing why major outputs changed.

That vertical slice becomes the foundation for the larger world simulation.

---

**Architecture status:** Implementation baseline pending project-owner acceptance.  
**Next recommended document:** `ADR-001-deterministic-discrete-time-kernel.md`.
