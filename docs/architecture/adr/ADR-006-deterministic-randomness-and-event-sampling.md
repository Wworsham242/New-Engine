# ADR-006: Deterministic Randomness and Event Sampling

**Status:** Proposed  
**Date:** 2026-09-17  
**Decision owners:** Project architecture  
**Applies to:** New Engine simulation kernel, subsystem models, event systems, AI support systems  
**Related:** `ADR-001-deterministic-discrete-time-kernel.md`, `ADR-002-authoritative-state-ownership-and-typed-contracts.md`, `ADR-003-equation-registry-units-cadence-and-provenance.md`, `ADR-004-solver-groups-convergence-and-numerical-stability.md`, `ADR-005-delayed-effects-pipelines-buffers-and-expectations.md`, `../SYSTEM_ARCHITECTURE_V0_1.md`

## Context

New Engine is deterministic by design.

Deterministic does not mean that every modeled process must be non-stochastic.

Some systems may legitimately require sampled events or probabilistic outcomes, such as:

- weather realization around a climate distribution;
- equipment failure;
- accident occurrence;
- combat hit/miss or damage variation at sufficiently detailed fidelity;
- disease transmission in a stochastic profile;
- political event timing where the model intentionally includes uncertainty;
- discovery/exploration events;
- project delay risk;
- disaster realization;
- AI exploration in non-authoritative planning modes.

The challenge is to permit stochastic behavior without allowing:

- replay divergence;
- thread-order dependence;
- hidden global RNG state;
- call-order dependence across unrelated systems;
- save/load divergence;
- non-reproducible bug reports;
- accidental gameplay changes because code was refactored;
- nondeterministic CI failures.

This ADR defines the random-number architecture, stream identity, event sampling rules, replay behavior, and deterministic tie-breaking requirements.

---

# Decision

## 1. Randomness is explicit model input

A stochastic draw is treated as an explicit model operation.

No authoritative simulation code may call:

```text
thread_rng()
rand::random()
OS entropy
system time
```

during simulation execution.

Randomness is supplied through deterministic kernel services.

---

## 2. One versioned authoritative PRNG family

The authoritative simulation uses one explicitly selected and versioned PRNG family.

The initial implementation should prefer a counter-based or splittable generator suitable for keyed independent draws.

Required properties:

- deterministic output;
- stable algorithm specification;
- reproducible sequence;
- cheap independent substreams;
- no dependence on thread scheduling;
- good statistical quality for simulation;
- practical Rust implementation.

---

## 3. Initial PRNG direction

Preferred family:

```text
ChaCha-based deterministic RNG
```

or another well-supported stable Rust implementation with explicit version pinning.

The exact crate/algorithm is an implementation decision, but once selected it becomes part of the authoritative model version.

A later ADR or implementation note should record the concrete choice.

---

## 4. Root seed

Every simulation run has a root seed.

```rust
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct RootSeed(pub [u8; 32]);
```

The root seed is recorded in:

- replay header;
- save files;
- experiment metadata;
- crash/bug reports where appropriate.

---

## 5. Random draws are keyed, not globally sequential

The default authoritative model should avoid one mutable global RNG sequence.

Preferred conceptual key:

```text
root_seed
+
subsystem_id
+
tick
+
entity_id
+
event_kind
+
draw_index
```

This means an unrelated new random draw in one subsystem should not shift all later random outcomes in another subsystem.

---

## 6. Random key type

Conceptually:

```rust
pub struct RandomKey {
    pub subsystem: SubsystemId,
    pub tick: Tick,
    pub entity: EntityId,
    pub event: RandomEventKindId,
    pub draw_index: u32,
}
```

The implementation may hash/encode this into a seed/counter.

---

## 7. Random event kinds are registered

Random operations have stable event-kind IDs.

```rust
pub struct RandomEventKindId(pub u32);
```

Examples:

```text
weather.monthly_precipitation
military.equipment_failure
infrastructure.accident
project.delay
disease.transmission
```

Stable human-readable keys should also exist for diagnostics.

---

## 8. Draw index is local to the event key

If an event requires multiple draws:

```text
draw_index = 0
draw_index = 1
draw_index = 2
```

The draw index is explicit.

Do not advance an opaque shared RNG state.

---

## 9. Thread independence

The same random key must produce the same draw regardless of:

- thread count;
- task scheduling;
- CPU timing;
- parallel execution order.

This is mandatory.

---

## 10. Refactoring resilience

Where practical, a draw's result should not change simply because unrelated code elsewhere added another random draw.

Keyed random access provides this stability.

---

## 11. Deterministic sampling API

Conceptual service:

```rust
pub trait RandomService {
    fn uniform01(&self, key: RandomKey) -> f64;

    fn uniform_u64(
        &self,
        key: RandomKey,
        upper_exclusive: u64,
    ) -> u64;

    fn bernoulli(
        &self,
        key: RandomKey,
        probability: f64,
    ) -> bool;
}
```

Additional distributions may be layered on top.

---

## 12. Distribution helpers are versioned

Supported helpers may include:

- uniform;
- Bernoulli;
- normal;
- lognormal;
- Poisson;
- categorical;
- weighted choice.

Distribution algorithm implementation affects reproducibility and therefore must be versioned/pinned.

---

## 13. Floating random conversion is stable

The conversion from raw PRNG bits to `f64` must be deterministic and documented.

Avoid relying on unspecified behavior in third-party helper functions unless the dependency is pinned and accepted.

---

## 14. Probability validation

Probabilities must satisfy:

```text
0 <= p <= 1
```

Invalid values are errors or deterministic clamps only where explicitly configured.

NaN probability is always an error.

---

## 15. Randomness is used only where model semantics justify it

Do not add stochasticity merely to make the simulation feel less predictable.

Randomness should represent modeled uncertainty or heterogeneous micro-outcomes.

---

## 16. Structural relationships remain deterministic by default

Examples that should normally remain deterministic:

- accounting identities;
- stock-flow balances;
- tax calculations;
- construction progress given known inputs;
- population accounting;
- capacity constraints;
- contract timing.

---

## 17. Stochastic realization should sit on deterministic probability models

Example:

```text
equipment failure probability
=
function(age, wear, maintenance, environment)
```

Then:

```text
failure_event
=
Bernoulli(probability)
```

The probability itself should arise causally.

---

## 18. Stochastic weather example

A weather model might calculate:

```text
expected rainfall
variance
climate state
seasonality
```

Then sample realization using a keyed draw.

The underlying climate trend remains deterministic given inputs.

---

## 19. Deterministic mode option

For development and certain research experiments, the engine may provide:

```text
stochastic realization disabled
```

where stochastic events are replaced with expected-value behavior when mathematically meaningful.

This mode must be a different recorded simulation profile.

---

## 20. Expected-value substitution is not always valid

Example:

```text
30% chance bridge fails
```

Replacing this with:

```text
bridge capacity -= 30%
```

may be physically wrong.

Therefore deterministic expected-value mode is optional and domain-specific.

---

## 21. Event sampling cadence is explicit

A probability must correspond to a defined period.

Example:

```text
monthly failure probability
```

is not interchangeable with:

```text
annual failure probability
```

Equation metadata must encode cadence.

---

## 22. Hazard conversion

Where models provide continuous/annual hazard but simulation samples monthly, conversion must be explicit.

Example:

```text
p_month =
1 - exp(-lambda_annual / 12)
```

or another model-appropriate transformation.

Do not divide probabilities naively unless mathematically justified.

---

## 23. Save/load requires no hidden RNG cursor

Because keyed draws are preferred, save state should not require one global RNG cursor.

If any subsystem uses a sequential local RNG stream, its state must be serialized explicitly.

---

## 24. Sequential local streams are discouraged

They may be permitted for algorithms that intrinsically consume a sequence.

If used:

- stream ownership is explicit;
- state is serialized;
- stream ID is stable;
- execution order is fixed.

Keyed draws remain preferred for authoritative events.

---

## 25. Replay requirements

Replay records:

- root seed;
- PRNG algorithm/version;
- distribution-helper version;
- model-set hash.

It does not need to record every draw if draws are deterministically regenerated.

---

## 26. Optional draw logging

Debug mode may record sampled events.

Example:

```text
tick 182
event project.delay
entity project_391
p = 0.08
u = 0.031
result = delayed
```

This is diagnostic only.

---

## 27. Random draws do not change from logging

Enabling diagnostics must not consume additional authoritative random draws.

---

## 28. Random service is read-only

Subsystems receive immutable access:

```rust
pub struct SolveContext<'a> {
    pub random: &'a dyn RandomService,
    // ...
}
```

No subsystem owns global random state.

---

## 29. Random event metadata

ADR-003 registry may include:

```rust
pub struct RandomEventMetadata {
    pub id: RandomEventKindId,
    pub key: &'static str,
    pub owner: SubsystemId,
    pub cadence: EquationCadence,
    pub provenance: &'static [ProvenanceTag],
}
```

---

## 30. Key completeness

A random key must include enough identity to avoid accidental reuse.

Bad:

```text
(subsystem, tick)
```

if thousands of entities draw the same event.

Better:

```text
(subsystem, tick, entity, event_kind, draw_index)
```

---

## 31. Entityless global events

For global events, use an explicit global entity/scope identifier.

Do not omit identity ambiguously.

---

## 32. Random keys are canonical

The binary encoding/hashing of random keys must be stable.

Do not hash raw Rust struct memory with padding.

Use explicit canonical serialization.

---

## 33. Hash-domain separation

Different random purposes should use domain-separated hashes/seeds.

Example:

```text
"weather"
"combat"
"project_delay"
```

This prevents accidental stream overlap.

---

## 34. Random-key hashing

A cryptographic hash such as BLAKE3 may be used to derive subkeys from the root seed.

Conceptually:

```text
subkey =
BLAKE3(
    root_seed
    || domain
    || canonical_random_key
)
```

Then initialize the selected PRNG/counter from that subkey.

Exact implementation may vary.

---

## 35. Performance

Keyed RNG must be fast enough for large simulations.

Optimization options:

- precomputed subsystem domain keys;
- counter-based generators;
- batched sampling;
- per-entity deterministic stream derivation.

Performance optimizations must not alter output.

---

## 36. Randomness in parallel loops

Pattern:

```text
for entity in parallel:
    key = deterministic(entity, tick, event)
    sample(key)
    write to entity slot
```

This is safe.

---

## 37. No atomic RNG counters

Do not use:

```text
AtomicU64 global draw counter
```

for authoritative simulation.

Thread interleaving would affect draw identity.

---

## 38. Weighted choice

Weighted sampling must use deterministic ordering of candidates.

Before sampling:

```text
candidate order = stable ID order
```

Weights align with that order.

---

## 39. Deterministic tie-breaking before randomness

If a deterministic rule fully resolves a tie, use it.

Randomness should not replace a clear domain rule.

Example:

```text
allocate to highest priority
then lowest stable ID
```

unless random selection is itself part of the model.

---

## 40. Random tie-breaking when semantically appropriate

If equal agents genuinely have no modeled priority and randomized allocation is desired:

```text
use keyed random tie-break
```

with stable candidate ordering.

---

## 41. AI randomness

Authoritative AI actions that use randomness must follow the same deterministic keyed system.

Examples:

- strategy exploration;
- tie-breaking between equivalent plans.

---

## 42. Non-authoritative AI tools

Developer assistants, search heuristics, or UI suggestion systems may use separate nondeterministic randomness if their outputs do not directly alter authoritative state.

Once an action is accepted into authoritative simulation, it is recorded explicitly.

---

## 43. Combat randomness

Combat may be modeled at different fidelity levels.

High-level deterministic combat is acceptable.

If stochastic combat is used, random draws must be keyed by stable identities such as:

```text
engagement
substep
attacker
defender
event_kind
draw_index
```

---

## 44. Combat draw volume should be controlled

Avoid millions of arbitrary micro-draws if aggregate distributions can represent the same process more efficiently.

Stochastic fidelity must justify computational cost.

---

## 45. Equipment failure

Example key:

```text
subsystem = Military
tick = 441
entity = vehicle_123
event = equipment_failure
draw_index = 0
```

Probability derives from:

- age;
- wear;
- maintenance;
- environment.

---

## 46. Project delay sampling

A construction project may have:

```text
delay probability
severity distribution
```

The draw occurs at a defined cadence.

A sampled delay becomes authoritative project state.

---

## 47. Disaster sampling

Disasters should generally separate:

```text
hazard probability
exposure
vulnerability
realized damage
```

Randomness may determine realization/intensity while physical vulnerability determines consequences.

---

## 48. Correlated randomness

Some events are spatially or temporally correlated.

Independent per-entity draws would be wrong.

Examples:

- regional drought;
- financial shock;
- epidemic wave.

Correlated processes should create a shared stochastic factor/event first, then deterministic or conditionally stochastic local effects.

---

## 49. Shared shock factor

Example:

```text
regional rainfall anomaly
```

sample once for region/month.

Then local crop equations consume that anomaly.

Do not sample unrelated rainfall independently for every farm abstraction if the intended weather system is regional.

---

## 50. Hierarchical stochastic processes

A model may sample:

```text
global factor
 -> regional factor
 -> local factor
```

Keys must reflect hierarchy.

---

## 51. Correlation is model logic

Do not attempt to create correlation accidentally by reusing the same random seed.

Use explicit shared latent variables/factors.

---

## 52. Temporal autocorrelation

Some stochastic variables persist.

Example:

```text
drought conditions
```

Use stateful processes such as AR models rather than independent monthly draws where appropriate.

---

## 53. Stateful stochastic process

Example:

```text
x[t+1]
=
rho * x[t]
+
epsilon[t]
```

`epsilon[t]` is a keyed random innovation.

`x[t]` is authoritative subsystem state.

---

## 54. Random innovations are distinct from expectation errors

Expectation models may use stochastic shocks, but the expected state itself remains owned state.

---

## 55. Distribution truncation

If a distribution is physically bounded, use explicit truncation or appropriate distribution.

Avoid sampling impossible values and silently clamping large errors.

---

## 56. Randomness and constraints

Sampled candidate values still pass through domain validation.

Example:

```text
damage fraction <= 1
```

---

## 57. Rare events

Rare-event probabilities should be numerically stable.

Avoid floating underflow or repeated naive checks where a hazard-process formulation is better.

---

## 58. Event arrival processes

For some events, Poisson/exponential arrival models may be appropriate.

Example:

```text
industrial accident arrivals
```

The algorithm must be deterministic for a given key/seed.

---

## 59. Event count sampling

If multiple events can occur in one period:

```text
count ~ Poisson(lambda)
```

may be preferable to repeated Bernoulli trials.

---

## 60. Repeated draws must have stable indexing

If event count is `N`, subevents use:

```text
draw_index 1..N
```

or child event IDs derived deterministically.

---

## 61. Child event identity

A sampled parent event can generate deterministic child IDs.

Example:

```text
storm_2029_08_region_4
```

with derived local damage events.

---

## 62. Randomness is not provenance

Provenance explains why a model/formula exists.

Random seed explains one realization.

Both should be tracked separately.

---

## 63. Scenario seed control

Scenario authoring may specify a seed.

If omitted, the launcher may generate one before simulation start.

Once the run begins, the resolved seed becomes authoritative metadata.

---

## 64. Experiment design

Controlled baseline-vs-shock experiments should normally use the same root seed.

This creates common random numbers and reduces noise when comparing interventions.

---

## 65. Common-random-number principle

Baseline and shock:

```text
same seed
same random-key scheme
```

so unrelated stochastic realizations remain aligned where possible.

This improves causal comparison.

---

## 66. Shock-induced entity differences

If a shock creates/removes entities, some later random-key sets may differ.

This is acceptable.

Keyed draws still prevent unrelated global sequence drift.

---

## 67. Monte Carlo experiments

Research mode may run many seeds:

```text
seed 1
seed 2
...
seed N
```

Outputs can report distributions.

Each run remains individually deterministic.

---

## 68. Gameplay default seed

A normal game may generate one random root seed at world creation.

Replay records it.

---

## 69. User-visible seed option

Developer/research UI should allow explicit seed entry.

This is useful for reproducing scenarios.

---

## 70. Seed confidentiality is not required

The root seed is not a security secret.

Do not treat PRNG as cryptographic security for anti-cheat without separate design.

---

## 71. Multiplayer implications

If multiplayer becomes authoritative lockstep, all peers must share:

- seed;
- build/model-set hash;
- actions;
- deterministic random algorithm.

This ADR is compatible with lockstep but does not define networking.

---

## 72. Save branching

If a player reloads an old save and takes different actions:

- unchanged random keys produce unchanged draws;
- new event keys produce new relevant draws.

This may create recognizable repeated events.

A future gameplay policy may optionally derive a branch seed, but that changes replay semantics and must be explicit.

---

## 73. No reroll-on-reload by default

Default behavior:

> loading the same save and making the same choices reproduces the same stochastic outcomes.

This is required for determinism.

---

## 74. Branch seed policy

If a game mode wants alternate randomness after branching, it must explicitly create:

```text
new root seed / branch seed
```

and record it.

Not v0.1 default.

---

## 75. Randomness and hidden information

A result may be deterministic internally but hidden from the player until revealed.

Uncertainty in player knowledge does not always require stochastic simulation.

---

## 76. Fog of war

Perception uncertainty may use:

- deterministic estimation error;
- stochastic observation noise;
- delayed information.

If stochastic, observation draws follow the same keyed architecture.

---

## 77. Perceived vs authoritative state

Future architecture may distinguish:

```text
authoritative true state
perceived state
```

Random observation error should modify perceived state, not authoritative reality.

---

## 78. Random event debugging

Developer command should eventually allow:

```text
show random draw
```

by:

- tick;
- subsystem;
- entity;
- event kind;
- draw index.

---

## 79. RNG audit tool

Potential Cargo command:

```powershell
cargo run -p sim-tools -- rng audit
```

Checks:

- duplicate random event keys;
- invalid event metadata;
- unstable candidate ordering;
- sequential RNG use in authoritative code.

---

## 80. Linting goal

Project conventions should discourage direct use of third-party RNG APIs outside the approved random module.

A custom wrapper crate/module should centralize authoritative randomness.

---

## 81. Suggested crate direction

Possible:

```text
crates/
  sim-random/
```

Responsibilities:

- root seed type;
- random key types;
- PRNG implementation;
- distribution helpers;
- event-kind registry support;
- canonical derivation;
- tests.

This is sufficiently cross-cutting to justify a dedicated crate once implementation begins.

---

## 82. Dependency direction

Subsystem crates depend on:

```text
sim-random API/types
```

They should not depend directly on the underlying RNG crate where practical.

This makes future algorithm migration manageable.

---

## 83. Version pinning

`Cargo.lock` must be committed for the authoritative workspace/application.

Randomness-related crate versions are pinned.

---

## 84. PRNG migration

Changing PRNG algorithm changes future realizations.

Therefore a PRNG change requires:

- random-system version bump;
- model/replay compatibility decision;
- migration note;
- golden test updates.

Old replays may require old PRNG compatibility or explicit rejection.

---

## 85. Distribution-helper migration

Changing normal/Poisson sampling algorithm can also change results.

Distribution implementation version is part of replay compatibility.

---

## 86. Canonical test vectors

`sim-random` must include fixed test vectors.

Example:

```text
root seed = known value
key = known value
expected u64 = known result
expected uniform01 = known result
```

These protect against accidental changes.

---

## 87. Cross-thread test

Test same batch of keys under:

```text
1 thread
2 threads
8 threads
24 threads
```

Results must match.

---

## 88. Enumeration-order test

Random results associated with entity IDs must remain the same even if processing order is reversed.

---

## 89. Save/load test

Generate state, save, load, continue.

Future sampled events must match uninterrupted run.

---

## 90. Refactor test

Where feasible, tests should ensure adding an unrelated random event kind does not alter existing event draws.

---

## 91. Baseline-vs-shock test

Same seed:

```text
baseline
shock
```

Verify unchanged entities/events retain same keyed draws where their keys remain identical.

---

## 92. Statistical tests

Determinism tests are primary.

Basic statistical sanity tests may verify distributions approximately behave as expected.

Do not overfit CI to fragile statistical thresholds.

---

## 93. Performance benchmark

Benchmark:

- millions of keyed uniform draws;
- Bernoulli draws;
- parallel draw batches.

Ensure random derivation is not a major bottleneck.

---

## 94. Randomness in scenario scripts

Scenario files may specify:

```text
deterministic event
```

or:

```text
hazard/probability model
```

They should not embed arbitrary nondeterministic script calls.

---

## 95. Data-driven probability

Scenario/model data can define:

- hazard rate;
- variance;
- distribution parameters.

The random engine remains centralized.

---

## 96. Security randomness is separate

If the application needs cryptographic randomness for:

- network tokens;
- authentication;
- secure IDs;

that uses a separate security RNG and does not enter simulation state.

---

## 97. Wall-clock time prohibition

Authoritative random keys must not include:

- current system time;
- frame time;
- thread ID;
- process ID.

---

## 98. Machine-identity prohibition

Random outcomes must not depend on:

- CPU serial;
- hostname;
- memory address;
- operating-system-specific hash randomization.

---

## 99. HashMap-order prohibition

Candidate sets for weighted/random selection must be sorted or stored canonically before sampling.

---

## 100. Deterministic shuffle

If a shuffle is needed:

- use approved random service;
- start from canonical order;
- use a versioned deterministic shuffle algorithm.

---

## 101. Random sampling during solver iteration

Avoid resampling stochastic events on every numerical iteration.

If a stochastic realization belongs to tick `t`, sample it once for that tick/event identity.

Solver iterations consume the same realization.

---

## 102. Frozen random realization per tick

Example:

```text
weather shock for month
```

is sampled before relevant solver iteration and remains fixed through convergence.

This prevents solver oscillation from consuming new randomness.

---

## 103. Stochastic innovations as exogenous inputs to solver

Conceptually:

```text
sample innovation
 -> freeze realization
 -> solve deterministic response
```

This is the preferred pattern.

---

## 104. Pipeline randomness

A project-delay realization should normally occur at defined checkpoints, not every solver pass.

---

## 105. Random event creation

If a random draw creates a delayed event/pipeline effect, that resulting event becomes ordinary authoritative state under ADR-005.

---

## 106. Randomness and causal tracing

Causal trace can record:

```text
probability model
sampled realization
downstream consequences
```

Example:

```text
storm occurred because sampled hazard event at tick 120
damage severity derived from storm intensity and infrastructure vulnerability
```

---

## 107. Explainability

Player-facing explanations should not expose raw PRNG internals unless in developer mode.

They may say:

```text
A severe regional drought occurred.
```

Developer trace may include probability and draw.

---

## 108. Provenance of stochastic models

Random event probability models still carry ADR-003 provenance tags.

Example:

```text
PublicLiterature
DatasetDerived
OriginalDesign
```

---

## 109. No randomness to hide model uncertainty

If a parameter is poorly known, do not automatically randomize it during gameplay.

Calibration uncertainty is different from modeled stochasticity.

Research mode may sample parameter uncertainty separately.

---

## 110. Parameter uncertainty experiments

Monte Carlo calibration studies may draw parameter sets at run start.

Those draws are recorded in experiment metadata.

Within a run, parameters then remain fixed unless modeled endogenously.

---

## 111. Epistemic vs aleatory uncertainty

The architecture distinguishes:

```text
epistemic uncertainty:
    uncertainty about model/parameter

aleatory uncertainty:
    modeled random process
```

Do not conflate them.

---

## 112. Deterministic AI planning simulations

AI may run hypothetical futures.

If those futures use stochastic events, planning should use:

- fixed scenario seeds;
- sampled scenario sets;
- expected-value approximations.

Planning randomness must not consume authoritative world RNG state.

---

## 113. AI simulation namespaces

Planning simulations use separate random domains/seeds.

They cannot perturb authoritative event draws.

---

## 114. Test-mode forced outcomes

Developer tools may force:

```text
event occurs
event does not occur
```

for testing.

Forced outcomes are explicit scenario/test overrides and recorded.

---

## 115. Mock random service

Unit tests can inject:

```rust
pub struct MockRandomService;
```

to return controlled values.

This improves deterministic equation tests.

---

## 116. API misuse prevention

Prefer APIs that require a complete `RandomKey`.

Avoid convenience functions like:

```rust
random_bool(p)
```

with implicit context.

---

## 117. Event-kind naming

Use namespaced keys:

```text
energy.facility_failure
military.weapon_malfunction
environment.storm_occurrence
```

---

## 118. Draw-index naming

When an event requires multiple logical draws, helper enums may prevent magic indices.

Example:

```rust
enum StormDraw {
    Occurrence = 0,
    Intensity = 1,
    Track = 2,
}
```

---

## 119. Random stream collision tests

Canonical derivation should make collisions cryptographically negligible.

Logical key duplication is still an architecture bug and should be detectable in debug tooling where practical.

---

## 120. Acceptance criteria

ADR-006 is implemented when:

1. `sim-random` or equivalent authoritative random module exists;
2. root seed type exists;
3. keyed random API exists;
4. no authoritative code uses OS/thread RNG directly;
5. stable random event-kind IDs exist;
6. random-key canonical encoding exists;
7. replay records RNG algorithm/version and seed;
8. test vectors exist;
9. thread-count independence test passes;
10. reverse-iteration-order test passes;
11. save/load random continuity test passes;
12. baseline-vs-shock common-random-number test passes;
13. solver iteration does not resample frozen tick events;
14. `cargo test --workspace` passes.

---

## 121. Initial implementation sequence

### Step 1

Create random types:

```text
RootSeed
RandomKey
RandomEventKindId
```

### Step 2

Implement canonical key encoding.

### Step 3

Select/pin authoritative PRNG.

### Step 4

Implement:

```text
uniform_u64
uniform01
bernoulli
```

### Step 5

Add fixed test vectors.

### Step 6

Add thread-independence tests.

### Step 7

Integrate random service into `SolveContext`.

### Step 8

Implement one stochastic demonstration:

```text
project delay
```

or:

```text
equipment failure
```

### Step 9

Add replay metadata.

---

## 122. Consequences

### Positive

- stochastic events remain exactly reproducible;
- parallelism cannot reorder outcomes;
- unrelated refactors do not globally shift random sequences;
- baseline-vs-shock experiments become cleaner;
- save/load remains stable;
- stochastic processes remain inspectable;
- authoritative AI randomness follows the same rules.

### Costs

- random keys require explicit identity;
- distribution implementations must be versioned;
- some existing Rust convenience APIs cannot be used directly;
- correlated stochastic systems require deliberate modeling;
- PRNG migration becomes a compatibility event.

These costs are accepted.

---

## 123. Rejected alternatives

### One global mutable RNG

Rejected because call order and parallel scheduling would alter all later outcomes.

### OS/thread RNG during simulation

Rejected because replay would not be deterministic.

### Record every random draw in replay

Rejected as the default because keyed regeneration is more compact and robust.

### Randomize every uncertain model parameter

Rejected because parameter uncertainty is not the same as modeled stochasticity.

### Resample during each solver iteration

Rejected because numerical iteration does not represent repeated passage of time or repeated worlds.

---

## 124. Open implementation questions

Still open:

- exact PRNG crate/algorithm;
- exact keyed derivation method;
- whether BLAKE3 is used for key derivation;
- exact normal/Poisson implementation;
- whether all random event IDs live in a checked-in manifest;
- whether stochastic weather is included in the first vertical slice;
- whether branch seeds are ever exposed as a gameplay option.

These do not change the core decision.

---

# Decision summary

New Engine will permit stochastic processes while remaining fully replayable.

Authoritative randomness uses:

- one versioned PRNG family;
- a recorded root seed;
- deterministic keyed random access;
- stable subsystem/entity/event identities;
- explicit draw indices;
- thread-independent sampling;
- fixed stochastic realizations during solver iteration;
- canonical candidate ordering;
- deterministic replay and save/load behavior.

Randomness is used only where the model intentionally represents stochastic behavior.

The simulation remains deterministic in the engineering sense:

> the same build, model, initial state, seed, and actions produce the same world.
