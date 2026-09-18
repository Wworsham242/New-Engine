# ADR-004: Solver Groups, Convergence, and Numerical Stability

**Status:** Proposed  
**Date:** 2026-09-17  
**Decision owners:** Project architecture  
**Applies to:** New Engine simulation kernel and subsystem solvers  
**Related:** `ADR-001-deterministic-discrete-time-kernel.md`, `ADR-002-authoritative-state-ownership-and-typed-contracts.md`, `ADR-003-equation-registry-units-cadence-and-provenance.md`, `../SYSTEM_ARCHITECTURE_V0_1.md`

## Context

ADR-001 established deterministic discrete-time execution, a monthly master tick, subsystem-specific cadences, Jacobi-style outer coupling, and bounded convergence.

ADR-002 established single-owner authoritative state and typed immutable contracts.

ADR-003 established first-class equation metadata, solver-group registration, cadence, units, temporal semantics, and provenance.

The next architectural requirement is to define the actual numerical behavior of iterative models.

New Engine contains tightly coupled feedback systems such as:

```text
Energy price
  <-> Energy demand
  <-> Production
  <-> Imports/exports
  <-> Stocks
  <-> Shortages
  <-> Investment
```

and:

```text
GDP
  <-> Income
  <-> Consumption
  <-> Investment
  <-> Employment
  <-> Tax revenue
  <-> Government spending
```

and:

```text
Food production
  <-> Stocks
  <-> Prices
  <-> Demand
  <-> Trade
  <-> Farmer investment
  <-> Yield/capacity
```

These relationships can oscillate, diverge, converge slowly, or become numerically unstable if solver behavior is not explicit.

The engine must:

- remain deterministic;
- avoid hidden order dependence;
- detect non-convergence;
- prevent silent explosions;
- support local subsystem solvers;
- support bounded outer coupling across subsystems;
- preserve physical/accounting constraints;
- emit useful diagnostics;
- remain performant enough for large-world simulation.

This ADR defines the solver-group architecture and convergence policy.

---

# Decision

## 1. Solver groups are explicit architecture objects

Iterative feedback is organized into named solver groups.

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SolverGroupId(pub u16);
```

Each group has:

- stable identity;
- owner or multi-subsystem scope;
- registered variables;
- registered equations;
- convergence variables;
- convergence policy;
- damping policy;
- iteration cap;
- fallback behavior.

---

## 2. Solver group categories

The initial architecture defines three categories.

```rust
pub enum SolverGroupKind {
    Local,
    OuterCoupled,
    ConstraintReconciliation,
}
```

### Local

Contained inside one subsystem.

Examples:

```text
energy.market
agriculture.market
economy.production_income
governance.fiscal
military.logistics
```

### OuterCoupled

Coordinates multiple subsystems through immutable contract snapshots.

Initial example:

```text
core.macro_energy
```

Potential members:

- Economy;
- Energy;
- Agriculture;
- Governance;
- Infrastructure.

### ConstraintReconciliation

Applies deterministic final constraints after model iteration.

Examples:

- nonnegative stocks;
- budget identity;
- population conservation;
- land-use bounds;
- military equipment conservation.

Constraint groups are not substitutes for proper model equations.

---

## 3. Outer coupling remains Jacobi-style

ADR-001's decision remains binding.

At outer iteration `n`:

1. all participating subsystems read snapshot `C[n]`;
2. each computes its candidate outputs independently;
3. outputs are collected;
4. damping is applied deterministically;
5. `C[n+1]` is constructed at a barrier;
6. residuals are measured;
7. convergence is tested;
8. either stop or repeat.

Conceptually:

```text
                C[n]
      ___________|____________
     |      |       |         |
  Economy Energy Agriculture Governance ...
     |      |       |         |
     |______|_______|_________|
              |
        deterministic
           barrier
              |
             C[n+1]
```

No participating subsystem observes another subsystem's partial iteration output.

---

## 4. Local solvers may use Gauss-Seidel

Inside one subsystem, deterministic Gauss-Seidel is allowed when useful.

Example:

```text
Energy local iteration:
    update demand
    update stocks
    update shortage
    update price
    update imports
```

This is acceptable because:

- one subsystem owns the state;
- order is explicit and versioned;
- local iteration is isolated from foreign-state mutation.

The local algorithm must remain deterministic.

---

## 5. Solver metadata

```rust
pub struct SolverGroupMetadata {
    pub id: SolverGroupId,
    pub key: &'static str,
    pub kind: SolverGroupKind,
    pub phase: TickPhase,
    pub cadence: EquationCadence,
    pub convergence_variables: &'static [ConvergenceVariable],
    pub policy: SolverPolicy,
    pub provenance: &'static [ProvenanceTag],
}
```

---

## 6. Solver policy

```rust
pub struct SolverPolicy {
    pub min_iterations: u32,
    pub max_iterations: u32,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
    pub damping: DampingPolicy,
    pub oscillation: OscillationPolicy,
    pub divergence: DivergencePolicy,
    pub non_convergence: NonConvergencePolicy,
}
```

Policies are configuration/model data and participate in replay/model-set hashing.

---

## 7. Minimum iterations

A solver group may require a minimum number of iterations.

Example:

```text
min_iterations = 2
```

This can be useful when one iteration cannot propagate a complete feedback loop.

The default is:

```text
min_iterations = 1
```

---

## 8. Maximum iterations

Every iterative group has a hard deterministic cap.

No authoritative solver may iterate until "however long convergence takes."

Example ranges:

```text
small local group:     4-12
core outer group:      4-16
difficult local group: 8-32
```

Exact values are per group.

---

## 9. Convergence variables are explicit

A group does not compare every internal variable automatically.

It registers variables that represent meaningful stability.

Example Energy group:

```text
price index
shortage index
demand
stocks
capacity utilization
```

Example core macro group:

```text
real GDP
investment
employment
energy price index
energy shortage
government revenue
food price index
```

---

## 10. Convergence variable metadata

```rust
pub struct ConvergenceVariable {
    pub variable: VariableId,
    pub metric: ResidualMetric,
    pub absolute_tolerance: Option<f64>,
    pub relative_tolerance: Option<f64>,
    pub weight: f64,
}
```

Per-variable tolerances override group defaults where necessary.

---

## 11. Residual metrics

Initial supported metrics:

```rust
pub enum ResidualMetric {
    Absolute,
    Relative,
    Mixed,
    Scaled,
}
```

---

## 12. Absolute residual

```text
r_abs = |new - old|
```

Useful for values with a meaningful absolute threshold.

---

## 13. Relative residual

```text
r_rel =
    |new - old|
    / max(|old|, epsilon)
```

Useful for scale-varying quantities.

---

## 14. Mixed residual

Preferred default:

```text
converged if:

|new - old| <= abs_tol
OR

|new - old| / max(|old|, scale_floor) <= rel_tol
```

This avoids unstable percentages near zero.

---

## 15. Scaled residual

For values whose natural scale is known:

```text
r_scaled =
    |new - old| / characteristic_scale
```

Example:

```text
inventory difference / normal monthly demand
```

---

## 16. Group convergence rule

A solver group converges when:

```text
iteration >= min_iterations
AND
all required convergence variables satisfy tolerance
```

Weighted aggregate scores may be emitted for diagnostics but are not the default stop condition.

---

## 17. No hidden adaptive tolerance

Tolerance values do not silently loosen because a solver is struggling.

Any adaptive tolerance scheme must be explicit, deterministic, and separately approved.

v0.1 uses fixed tolerances per group/variable.

---

## 18. Damping is first-class

Damping reduces oscillation.

Basic form:

```text
published =
    old + alpha * (candidate - old)
```

where:

```text
0 < alpha <= 1
```

---

## 19. Damping policy

```rust
pub enum DampingPolicy {
    None,
    Fixed { alpha: f64 },
    DeterministicAdaptive(AdaptiveDampingPolicy),
}
```

v0.1 default:

```text
Fixed
```

Adaptive damping is permitted only if deterministic and versioned.

---

## 20. Fixed damping guidance

Typical initial values:

```text
1.00 -> no damping
0.75 -> mild damping
0.50 -> moderate damping
0.25 -> strong damping
```

Damping should be used only where needed.

It is not a substitute for correcting a bad model.

---

## 21. Adaptive damping

If later enabled, it may respond to deterministic signals such as:

- sign-flipping residual;
- residual growth;
- repeated overshoot.

Example conceptual rule:

```text
if oscillating:
    alpha = max(alpha * 0.5, alpha_min)
```

All state required to compute adaptive damping must be part of deterministic solver state.

---

## 22. Oscillation detection

The solver tracks recent residual directions for convergence variables.

Simple two-cycle detection:

```text
x[n] - x[n-1]
and
x[n-1] - x[n-2]

have opposite signs
with similar magnitude
```

This indicates potential oscillation.

---

## 23. Oscillation policy

```rust
pub struct OscillationPolicy {
    pub detect_two_cycle: bool,
    pub magnitude_ratio_min: f64,
    pub response: OscillationResponse,
}
```

```rust
pub enum OscillationResponse {
    DiagnosticOnly,
    ApplyConfiguredDamping,
    FailStrictRun,
}
```

Normal gameplay:

```text
ApplyConfiguredDamping
```

if adaptive damping is enabled.

Otherwise:

```text
DiagnosticOnly
```

---

## 24. Divergence detection

A solver is diverging when residual magnitude grows materially over consecutive iterations.

Example:

```text
r[n] > growth_factor * r[n-1]
```

for a configured number of iterations.

---

## 25. Divergence policy

```rust
pub struct DivergencePolicy {
    pub growth_factor: f64,
    pub consecutive_iterations: u32,
    pub response: DivergenceResponse,
}
```

```rust
pub enum DivergenceResponse {
    DiagnosticOnly,
    IncreaseDamping,
    RevertToBestIteration,
    AbortStrictRun,
}
```

v0.1 recommended gameplay response:

```text
RevertToBestIteration
```

after reaching the iteration cap.

---

## 26. Best-iteration tracking

The solver tracks the iteration with the smallest convergence score.

If the group fails to converge:

```text
best_iteration =
    iteration with lowest deterministic residual score
```

The group may commit the best stable candidate rather than blindly committing the last iteration.

This is preferable when the final iteration has begun diverging.

---

## 27. Deterministic residual score

For best-iteration tracking:

```text
score =
    sum(weight_i * normalized_residual_i)
```

Normalization is defined by each convergence variable's residual metric.

This score is diagnostic/fallback support.

The normal convergence rule still requires all required variables to satisfy tolerance.

---

## 28. Non-convergence policy

```rust
pub enum NonConvergencePolicy {
    CommitBestIterationWithDiagnostic,
    CommitLastIterationWithDiagnostic,
    AbortStrictRun,
}
```

New recommended gameplay default:

```text
CommitBestIterationWithDiagnostic
```

This refines ADR-001's simpler fallback.

Strict research/testing mode:

```text
AbortStrictRun
```

---

## 29. Fallback output must be bounded

A non-converged candidate still passes through:

- domain validation;
- physical bounds;
- accounting reconciliation;
- NaN/Infinity rejection.

Failure to satisfy hard invariants cannot be ignored.

---

## 30. Hard constraints vs soft convergence

Hard constraints include:

```text
stock >= 0
population >= 0
capacity >= 0
shares in valid range
equipment count >= 0
land use <= available land
```

Soft convergence includes:

```text
price settled
demand stabilized
investment stabilized
GDP stabilized
```

A simulation may continue with imperfect soft convergence.

It may not continue with invalid hard state unless a deterministic repair rule exists.

---

## 31. Constraint reconciliation happens after solver iteration

The main order is:

```text
solver iterations
 -> candidate state
 -> reconciliation
 -> validation
 -> commit
```

Constraint reconciliation does not occur unpredictably between every equation unless the local solver explicitly defines that behavior.

---

## 32. Some constraints belong inside local iteration

Example:

Energy stocks cannot go below zero while computing shortage.

That bound is integral to the local algorithm and should be enforced inside the solver.

The final reconciliation pass handles world-level consistency.

---

## 33. Accounting identities

Accounting identities should usually be solved exactly or within machine tolerance.

Examples:

```text
sources = uses
assets - liabilities = equity
government revenue - spending = fiscal balance
```

They should not use loose behavioral tolerances.

---

## 34. Market adjustment is not exact accounting

Market relationships may be disequilibrium processes.

Example:

```text
supply != demand
```

can be temporarily valid because stocks, queues, shortages, and prices mediate imbalance.

The architecture must not force every market to clear exactly each month.

---

## 35. Stocks are buffers

A stock variable may absorb disequilibrium.

Example:

```text
supply + imports - demand - exports
    -> inventory change
```

This often reduces the need for artificial exact equilibrium.

---

## 36. Shortage is a legitimate state

A shortage variable may remain positive at convergence.

Convergence means:

> the solver has reached a stable result for this tick,

not:

> all unmet demand vanished.

---

## 37. Local vs outer convergence

Local subsystem groups converge first against the current outer snapshot.

Example:

```text
outer C[n]

Energy local solve:
    iterate energy market
    produce EnergyContracts[n+1 candidate]

Economy local solve:
    iterate economy internals
    produce EconomyContracts[n+1 candidate]

barrier
outer convergence test
```

---

## 38. Local failure propagation

If a local solver does not converge:

- it applies its own fallback policy;
- publishes a flagged candidate;
- outer solver records the local failure;
- strict mode may abort.

Outer convergence does not erase local failure diagnostics.

---

## 39. Solver status type

```rust
pub enum SolverStatus {
    Converged,
    NonConverged,
    Diverging,
    Oscillating,
    InvalidState,
}
```

---

## 40. Solver result metadata

```rust
pub struct SolverResultMeta {
    pub status: SolverStatus,
    pub iterations: u32,
    pub best_iteration: u32,
    pub final_score: f64,
    pub best_score: f64,
    pub used_damping: f64,
}
```

This metadata may be stored in diagnostics, not authoritative world state.

---

## 41. Deterministic evaluation order

Within a solver group:

- equation order is fixed;
- variable iteration order is fixed;
- entity iteration order is fixed;
- reductions are fixed.

Parallel execution must preserve the same logical result.

---

## 42. Parallel local solves

A subsystem may parallelize across independent entities.

Example:

```text
country 1
country 2
country 3
...
```

if cross-country coupling is not required inside that step.

Outputs are written to deterministic slots.

---

## 43. Coupled country systems require explicit solver structure

Examples:

- bilateral trade;
- migration;
- financial contagion.

These cannot be parallelized as independent country calculations if they interact.

They require a declared coupling strategy.

---

## 44. Deterministic reductions

Forbidden:

```text
multiple threads add into shared f64 accumulator
```

Preferred:

```text
parallel per-index outputs
 -> ordered deterministic reduction
```

---

## 45. Floating-point reproducibility

ADR-001 policy remains:

> same authoritative build and platform target should produce deterministic results.

Solver algorithms must avoid:

- unordered reductions;
- race-dependent iteration;
- non-deterministic math paths.

---

## 46. Solver snapshots

Each outer iteration has an immutable snapshot identity:

```rust
pub struct SolverIterationId {
    pub tick: Tick,
    pub group: SolverGroupId,
    pub iteration: u32,
}
```

This supports tracing and debugging.

---

## 47. Intermediate state visibility

Intermediate iteration state is not authoritative.

It may be retained temporarily for:

- diagnostics;
- convergence comparison;
- best-iteration fallback;
- causal tracing.

It is discarded after commit unless tracing is enabled.

---

## 48. Convergence diagnostics

On failure or debug mode, emit top residuals.

Example:

```text
core.macro_energy did not converge

tick: 194
iterations: 12
best iteration: 9

largest residuals:
    energy.price_index      0.82%
    economy.investment      0.64%
    governance.revenue      0.31%
    agriculture.food_price  0.18%
```

---

## 49. Oscillation diagnostics

Example:

```text
energy.market oscillation detected:
    variable: energy.price_index
    pattern: +2.3%, -2.0%, +1.8%, -1.5%
```

---

## 50. Divergence diagnostics

Example:

```text
economy.production_income residual growth:
    0.04
    0.09
    0.21
    0.51
```

---

## 51. Solver tracing must be optional

Full iteration tracing can be expensive.

Modes:

```text
Off
FailuresOnly
Summary
Full
```

---

## 52. Solver metrics

The engine should track:

- iterations per group;
- convergence rate;
- non-convergence count;
- damping activation count;
- average residual;
- max residual;
- solve time.

Performance metrics do not affect authoritative state.

---

## 53. Convergence policy can vary by fidelity profile

A research profile may use:

```text
tighter tolerances
more iterations
```

A gameplay profile may use:

```text
moderate tolerances
lower iteration caps
```

The profile is recorded in replay metadata.

---

## 54. Fidelity must not silently alter equation semantics

Changing tolerances is acceptable.

Changing the actual model equations requires a different model-set version/profile identity.

---

## 55. Warm starts

A solver may initialize from previous committed state.

This is encouraged where appropriate.

Example:

```text
price[n=0] = previous month's price
```

Warm starts improve convergence and reflect system inertia.

---

## 56. Cold starts

World initialization may require dedicated initialization solves.

The first tick should not assume all previous state exists.

A separate initialization phase may run:

```text
calibration
 -> initial equilibrium / reconciliation
 -> committed tick 0
```

---

## 57. Initialization solver policy

Initialization may have:

- higher iteration caps;
- stricter diagnostics;
- special calibration constraints.

Initialization must still be deterministic.

---

## 58. Shock ticks

A large shock may temporarily worsen convergence.

Example:

```text
50% oil production loss
```

The solver should not artificially suppress the shock merely to force convergence.

Damping applies to iterative solver communication, not to the physical shock itself unless the model defines a gradual shock.

---

## 59. Exogenous shocks remain exact inputs

If scenario says:

```text
capacity = capacity * 0.5
```

that input is applied exactly.

Solver damping affects downstream iterative response.

---

## 60. Avoid convergence-induced artificial inertia

Strong damping can unintentionally delay real-world response.

Therefore:

- damping is minimized;
- damping is documented;
- physical/stock equations are not damped unless required;
- behavioral feedback is the primary target for damping.

---

## 61. Solver group boundaries should follow causal density

Use local solver groups when variables are tightly recursive.

Do not place an entire subsystem in one iterative group merely because it belongs to the same domain.

Example Energy may have separate groups:

```text
energy.short_run_market
energy.capacity_investment
energy.resource_depletion
```

---

## 62. Slow structural loops should not iterate every month unnecessarily

Example:

```text
education -> productivity -> income -> education spending
```

This can close over years.

It does not require monthly fixed-point iteration.

Time itself can resolve slow feedback.

---

## 63. Fast loops vs slow loops

Use iteration when feedback is assumed to resolve within the current synchronization period.

Use temporal lag when the real process takes time.

This is a central modeling rule.

---

## 64. Do not use solver iteration to hide missing lag structure

Bad:

```text
run 20 iterations until new factory capacity responds immediately to price
```

if factory construction should take years.

Correct:

```text
price signal
 -> investment decision
 -> construction pipeline
 -> capacity later
```

---

## 65. Same-tick vs lagged relationship test

For every feedback edge, ask:

```text
Could this causal response materially occur within the master period?
```

If yes:

```text
candidate for same-tick solver coupling
```

If no:

```text
use lag/pipeline state
```

---

## 66. Monthly master tick implications

With a monthly master tick:

Likely same-tick:

```text
fuel shortage -> spot price
price -> short-run consumption response
tax receipts -> monthly cash flow
```

Likely lagged:

```text
price -> refinery construction
unemployment -> fertility change
education spending -> workforce skill
military losses -> replacement platform delivery
```

---

## 67. Solver graph validation

ADR-003 registry tooling should identify:

- cycles;
- strongly connected components;
- solver-group membership.

A cycle outside an explicitly declared group or temporal lag is an error.

---

## 68. SCCs are architectural hints, not automatic solver groups

Observed IFs SCC analysis demonstrated this distinction.

A giant dependency SCC does not imply:

```text
solve all variables simultaneously
```

The engine uses:

- cadence;
- lag;
- subsystem ownership;
- physical process timing;
- explicit solver grouping.

---

## 69. Outer solver scope should remain small

The core outer group should expose only bridge variables necessary for convergence.

Do not compare every subsystem internal variable.

This improves performance and stability.

---

## 70. Candidate initial outer convergence set

Initial vertical slice:

```text
energy.price_index
energy.shortage_index
energy.demand
economy.real_gdp
economy.investment
governance.tax_revenue
governance.fiscal_balance
```

Agriculture and Infrastructure are added when their vertical slices exist.

---

## 71. Initial local Energy convergence set

```text
energy.price_index
energy.shortage_index
energy.demand
energy.stocks
```

---

## 72. Initial Economy convergence set

```text
economy.real_gdp
economy.consumption
economy.investment
economy.employment
```

---

## 73. Initial Governance convergence set

```text
governance.tax_revenue
governance.spending
governance.fiscal_balance
```

---

## 74. Convergence tolerances are dimension-aware

Do not use one raw numeric tolerance for all variables.

Examples:

```text
price index          -> relative
GDP                  -> relative
inventory quantity   -> mixed
population counts    -> absolute/relative
share/rate           -> absolute percentage-point style
```

---

## 75. Suggested initial default tolerances

These are starting engineering values, not economic truth:

```text
relative tolerance: 1e-4 to 1e-3
absolute tolerance: domain-specific
```

They must be validated through experiments and profiling.

---

## 76. Tolerance sensitivity testing

For important scenarios, test:

```text
loose tolerance
baseline tolerance
tight tolerance
```

If major outcomes materially change, the solver/model requires review.

---

## 77. Iteration-cap sensitivity testing

Similarly test:

```text
8
12
16
32
```

iterations for difficult groups.

The model should not rely on an arbitrary cap to determine qualitative outcomes.

---

## 78. Damping sensitivity testing

Test:

```text
alpha 1.0
alpha 0.75
alpha 0.5
```

If long-run results materially change rather than just convergence path, review the model.

---

## 79. Solver policies are versioned

Changes to:

- tolerance;
- damping;
- iteration cap;
- residual metric;
- fallback behavior;

change solver-policy version and model-set hash.

---

## 80. Validation mode

A strict validation profile should use:

```text
AbortStrictRun
```

on:

- local solver failure;
- outer solver failure;
- invalid state;
- NaN/Infinity;
- broken accounting identity.

This is intended for CI and research testing.

---

## 81. Gameplay mode

Gameplay may use:

```text
CommitBestIterationWithDiagnostic
```

for soft non-convergence.

Hard invalid states remain errors unless repaired deterministically.

---

## 82. Repair rules must be explicit

Example:

If tiny floating error produces:

```text
stock = -1e-12
```

a declared epsilon clamp may set it to zero.

If:

```text
stock = -1000
```

that is not a harmless rounding correction.

---

## 83. Numerical epsilon policies

Each domain may define epsilon thresholds.

Example:

```rust
pub struct NumericPolicy {
    pub zero_epsilon: f64,
    pub balance_epsilon: f64,
}
```

These are model configuration and versioned.

---

## 84. Constraint corrections are logged

Any reconciliation correction above a trivial epsilon should be diagnosable.

Example:

```text
Energy stock clamped:
    country: X
    before: -0.000000001
    after: 0
```

---

## 85. Solver group API direction

Conceptual trait:

```rust
pub trait SolverGroup {
    type Snapshot;
    type Candidate;

    fn initialize(
        &self,
        ctx: &SolveContext,
    ) -> Self::Snapshot;

    fn iterate(
        &self,
        snapshot: &Self::Snapshot,
        iteration: u32,
        ctx: &SolveContext,
    ) -> Self::Candidate;

    fn build_next_snapshot(
        &self,
        old: &Self::Snapshot,
        candidate: Self::Candidate,
        iteration: u32,
    ) -> Self::Snapshot;

    fn residuals(
        &self,
        old: &Self::Snapshot,
        new: &Self::Snapshot,
    ) -> ResidualSet;
}
```

Exact APIs may differ.

---

## 86. Avoid one generic solver trait for every numerical method

Some local solvers may need specialized APIs.

The architecture standardizes:

- metadata;
- convergence policy;
- diagnostics;
- determinism;
- fallback semantics.

It does not require every numerical algorithm to fit one overly abstract interface.

---

## 87. Solver-policy crate placement

Recommended:

```text
sim-equations:
    solver metadata IDs

sim-kernel:
    generic outer iteration machinery

subsystem-*:
    local solver implementation
```

Potential shared crate later:

```text
sim-solvers
```

if enough reusable algorithms emerge.

---

## 88. Cargo testing

Required commands:

```powershell
cargo test --workspace
cargo check --workspace
cargo clippy --workspace --all-targets
```

Solver-specific tests should be runnable independently.

---

## 89. Solver benchmark harness

Recommended future:

```text
cargo bench -p subsystem-energy
cargo bench -p subsystem-economy
```

Benchmark:

- cold start;
- warm start;
- shock tick;
- normal tick;
- worst-case iteration cap.

---

## 90. Regression fixtures

Keep controlled scenario fixtures:

```text
energy-shock-small
energy-shock-large
tax-shock
food-shock
```

Each records:

- convergence status;
- iteration counts;
- key output values;
- state hash.

---

## 91. Iteration-count regression

A model change that increases a common solver from:

```text
3 iterations -> 15 iterations
```

should trigger investigation even if final outputs match.

---

## 92. Non-convergence budget

Normal production runs should have near-zero recurring non-convergence in ordinary conditions.

A scenario that routinely hits the cap indicates:

- bad solver grouping;
- bad damping;
- missing lag structure;
- unstable equation;
- tolerance mismatch.

---

## 93. Crisis conditions may legitimately stress convergence

Examples:

- wartime collapse;
- hyperinflation;
- catastrophic supply shock;
- state failure.

But solver diagnostics must distinguish:

```text
extreme modeled state
```

from:

```text
numerical failure
```

---

## 94. Discontinuities

Some model transitions are discontinuous.

Examples:

- facility destroyed;
- embargo activated;
- mobilization law switched;
- government default event.

Do not force smooth convergence across a real discontinuity.

Apply the discrete event first, then solve the resulting state.

---

## 95. Threshold behavior

Thresholds are allowed when the modeled process has them.

Example:

```text
reserve mobilization if readiness < threshold
```

But threshold-triggered actions must be deterministic and order-defined.

---

## 96. Solver hysteresis

Where threshold chattering is a risk, use explicit hysteresis.

Example:

```text
turn emergency rationing on at shortage > 10%
turn off only when shortage < 5%
```

Hysteresis is model logic, not solver damping.

---

## 97. Complementarity-like constraints

Some markets may involve:

```text
quantity >= 0
price >= 0
shortage >= 0
```

Exact complementarity solvers are not required in v0.1.

Simpler deterministic adjustment models are acceptable initially.

---

## 98. Optimization solvers

Allocation problems may later use:

- linear programming;
- network flow;
- convex optimization.

Any external solver library used for authoritative state must satisfy deterministic-output requirements or be wrapped with deterministic tie-breaking.

---

## 99. Deterministic tie-breaking

Whenever several allocations are mathematically equivalent, tie-breaking must be stable.

Examples:

```text
lowest stable ID
predefined priority order
proportional split
```

Never rely on map iteration or solver-library arbitrary choice.

---

## 100. Military logistics solver

Military logistics may use a separate local solver due to:

- transport capacity;
- fuel;
- ammunition;
- route constraints;
- priority allocation.

It may run on internal weekly/daily substeps.

Its outputs aggregate to the monthly synchronization boundary.

---

## 101. Trade solver

Trade may evolve into its own coupled solver.

Potential levels:

```text
pooled global market
regional pools
bilateral network
```

The chosen implementation depends on fidelity profile.

Solver metadata must expose the active model.

---

## 102. Finance solver

If finance becomes sufficiently coupled:

```text
interest rates
debt service
credit availability
capital flows
exchange rates
```

it may require its own solver group.

Do not automatically bury all finance inside Economy.

---

## 103. Solver-group ownership

A local group has one subsystem owner.

An outer group is kernel-owned orchestration over subsystem contracts.

This distinction is important for crate dependencies.

---

## 104. Example Rust metadata

```rust
pub static CORE_MACRO_ENERGY_SOLVER: SolverGroupMetadata =
    SolverGroupMetadata {
        id: SolverGroupId(1),
        key: "core.macro_energy",
        kind: SolverGroupKind::OuterCoupled,
        phase: TickPhase::CoreCoupledSolve,
        cadence: EquationCadence::EveryMasterTick,
        convergence_variables: CORE_CONVERGENCE_VARS,
        policy: SolverPolicy {
            min_iterations: 2,
            max_iterations: 12,
            absolute_tolerance: 1e-8,
            relative_tolerance: 1e-3,
            damping: DampingPolicy::Fixed { alpha: 0.75 },
            oscillation: OscillationPolicy::default(),
            divergence: DivergencePolicy::default(),
            non_convergence:
                NonConvergencePolicy::CommitBestIterationWithDiagnostic,
        },
        provenance: &[ProvenanceTag::OriginalDesign],
    };
```

Values above are illustrative starting points.

---

## 105. Solver policy data should be configurable

Policies should be loadable from profile/config data while constrained by registered valid ranges.

Example:

```text
standard
research
fast
strict
```

---

## 106. Profile overrides are recorded

If a user selects:

```text
Fast simulation
```

and it reduces iterations/tolerance strictness, the replay records the exact solver profile.

---

## 107. AI decisions do not run inside numerical solver iteration

Strategic AI should normally act at defined decision phases.

It should not change policy every solver iteration.

Otherwise iteration may mix numerical stabilization with changing decisions.

---

## 108. Player actions are fixed during a tick solve

Actions accepted for the tick are applied at the relevant phase before iteration.

The solver computes consequences of those decisions.

---

## 109. Expectations models

Future expectation models may themselves be iterative.

Example:

```text
expected inflation
expected demand
expected energy price
```

They need explicit temporal semantics.

Do not let expectations become hidden self-referential loops.

---

## 110. Delayed expectations

Often expectations should use:

```text
rolling history
trend
policy signal
```

rather than current-iteration perfect foresight.

This is a modeling decision per equation.

---

## 111. Perfect-foresight solvers are not default

The engine is intended to model bounded real systems, not assume all actors solve the future global equilibrium each tick.

---

## 112. Causal explainability

Solver diagnostics should support explanations such as:

```text
Energy price remained elevated because:
    supply fell
    stocks were depleted
    imports were capacity constrained
    demand response was insufficient
```

The solver should not appear as an opaque numerical black box.

---

## 113. IFs refer-back

The solver architecture is informed by IFs observations that:

- many variables respond in the same broad period;
- subsystems exhibit dense reciprocal coupling;
- some relationships are delayed;
- giant dependency SCCs do not map cleanly to one solver block.

Adopted principle:

> Use explicit local iterative blocks and bounded cross-subsystem coupling, with time/lags handling slower feedback.

New Engine does not reproduce IFs solver internals.

---

## 114. Model review requirement

Any new solver group should document:

```text
Why must this loop resolve within one master period?
Why is temporal lag insufficient?
What are convergence variables?
What is the physical/economic meaning of convergence?
What is the fallback behavior?
```

---

## 115. Solver-group size discipline

Very large groups require justification.

A group containing hundreds of variables should trigger architectural review.

Large dependency SCCs alone are not justification.

---

## 116. Acceptance criteria

ADR-004 is implemented when:

1. `SolverGroupId` and metadata exist;
2. local and outer solver categories exist;
3. fixed damping is supported;
4. mixed residual convergence is implemented;
5. max iteration caps are enforced;
6. best-iteration fallback exists;
7. oscillation and divergence diagnostics exist;
8. strict-run abort mode exists;
9. local Energy solver exists;
10. Economy/Energy outer Jacobi loop exists;
11. deterministic solver tests pass;
12. tolerance sensitivity tests exist;
13. state hashes match across repeated runs;
14. solver metadata is visible through `sim-tools`.

---

## 117. Initial implementation sequence

### Step 1

Implement generic residual types:

```text
ResidualMetric
ConvergenceVariable
ResidualSet
```

### Step 2

Implement fixed-damping helper.

### Step 3

Implement generic outer Jacobi loop in `sim-kernel`.

### Step 4

Implement local Energy market loop.

### Step 5

Implement minimal Economy loop.

### Step 6

Connect Economy/Energy outer group.

### Step 7

Add solver diagnostics.

### Step 8

Add strict mode and best-iteration fallback.

### Step 9

Add sensitivity tests.

---

## 118. Consequences

### Positive

- feedback behavior is explicit;
- deterministic iteration semantics are preserved;
- oscillation and divergence are detectable;
- failure does not silently corrupt state;
- strong shocks can be handled without hiding them;
- slow feedback can remain temporal instead of being forced into giant solvers;
- local vs outer numerical structure remains clean;
- solver performance can be profiled independently.

### Costs

- more solver metadata;
- more testing;
- damping/tolerance calibration work;
- best-iteration storage;
- diagnostics overhead in development modes;
- some model groups may need redesign after stability testing.

These costs are accepted.

---

## 119. Rejected alternatives

### Iterate until convergence with no cap

Rejected because runtime becomes unpredictable and failure may hang the simulation.

### Commit the last iteration blindly

Rejected as the default because a diverging solver may end on its worst candidate.

### Force all markets to exact equilibrium

Rejected because shortages, inventories, queues, and disequilibrium are legitimate modeled states.

### Put all feedback in one global solver

Rejected because time constants and domain semantics differ.

### Use heavy damping everywhere

Rejected because it can create artificial inertia.

### Automatically convert every dependency cycle into a solver group

Rejected because many cycles close through time rather than within one tick.

---

## 120. Open implementation questions

Still open:

- exact initial tolerance values per variable;
- exact oscillation thresholds;
- exact divergence thresholds;
- whether best-iteration snapshots use full clone or selective contract/state capture;
- whether local solvers share a reusable `sim-solvers` crate;
- whether adaptive damping is included in v0.1 or deferred;
- whether external optimization libraries are used for allocation systems;
- exact trade solver approach;
- exact finance solver approach.

These do not change the main decision.

---

# Decision summary

New Engine will use **explicit deterministic solver groups** with:

- local subsystem iteration;
- Jacobi-style outer subsystem coupling;
- fixed iteration caps;
- explicit convergence variables;
- mixed absolute/relative residuals;
- deterministic damping;
- oscillation detection;
- divergence detection;
- best-iteration fallback;
- strict validation mode;
- hard constraint enforcement;
- stable deterministic execution order.

Iteration is used only when a feedback loop is assumed to resolve within the current synchronization period.

Slow real-world feedback is modeled through time, stocks, and pipelines rather than forced into a same-tick numerical equilibrium.

This ADR establishes the numerical-stability foundation for New Engine's coupled world simulation.
