# ADR-007: Save, Replay, State Hashing, and Compatibility

**Status:** Proposed  
**Date:** 2026-09-17  
**Decision owners:** Project architecture  
**Applies to:** New Engine simulation kernel, persistence layer, replay system, experiment framework  
**Related:** `ADR-001-deterministic-discrete-time-kernel.md`, `ADR-002-authoritative-state-ownership-and-typed-contracts.md`, `ADR-003-equation-registry-units-cadence-and-provenance.md`, `ADR-004-solver-groups-convergence-and-numerical-stability.md`, `ADR-005-delayed-effects-pipelines-buffers-and-expectations.md`, `ADR-006-deterministic-randomness-and-event-sampling.md`, `../SYSTEM_ARCHITECTURE_V0_1.md`

## Context

New Engine is designed around deterministic simulation and reproducible experiments.

Previous ADRs established:

- one authoritative committed world state per master tick;
- subsystem-owned state;
- deterministic scheduling and solver behavior;
- delayed pipelines and queues;
- deterministic stochastic event sampling;
- state hashing as a core validation mechanism;
- baseline-vs-shock experiment support.

Persistence must preserve these guarantees.

A save/replay system that merely serializes Rust structs is insufficient because:

- Rust memory layout is not a stable file format;
- crate/type refactors can invalidate old saves;
- unordered maps can serialize nondeterministically;
- floating-point state must be written canonically;
- active pipelines, event queues, expectations, solver policy, and RNG metadata must all survive reload;
- replays must detect model/data mismatches;
- corrupted or partially written files must be rejected safely;
- exact deterministic validation requires committed-state hashes.

This ADR defines the authoritative save, replay, snapshot, hashing, compatibility, and migration model.

---

# Decision

## 1. Save files and replay logs are versioned protocols

Persistence formats are explicit schemas.

They are not raw Rust memory dumps.

Each format has an independent version.

Conceptually:

```rust
pub struct SaveFormatVersion(pub u32);
pub struct ReplayFormatVersion(pub u32);
pub struct SnapshotFormatVersion(pub u32);
```

Format versions are separate from:

- engine version;
- model-set version;
- data version;
- PRNG version.

---

## 2. Authoritative persistence layers

New Engine distinguishes:

```text
Save
Replay
Snapshot
Checkpoint
Diagnostics
```

### Save

A complete resumable game/simulation state.

### Replay

Initial condition plus authoritative actions/inputs and verification metadata sufficient to regenerate the run.

### Snapshot

Canonical committed world state at a specific tick.

### Checkpoint

A snapshot stored periodically to accelerate replay seeking/resume.

### Diagnostics

Optional non-authoritative traces and performance records.

---

## 3. Saves contain authoritative state

A save must contain all state required to continue deterministically.

This includes:

- current committed tick;
- calendar/profile state;
- subsystem authoritative state;
- active pipelines;
- queues/backlogs;
- delayed events;
- expectation state;
- rolling-memory state needed by equations;
- maturity schedules;
- authoritative AI state if applicable;
- deterministic local sequential RNG state if any remains;
- player/policy state;
- registry mappings needed for stable IDs;
- required replay/version metadata.

---

## 4. Saves do not need to contain rebuildable caches

Derived caches should normally be excluded.

Examples:

- lookup caches;
- transient solver buffers;
- generated graph views;
- UI state;
- performance counters.

Caches are rebuilt after load.

---

## 5. Save header

Conceptual structure:

```rust
pub struct SaveHeader {
    pub magic: [u8; 8],
    pub save_format: SaveFormatVersion,
    pub engine_version: EngineVersion,
    pub architecture_version: ArchitectureVersion,
    pub model_set_hash: ModelSetHash,
    pub data_set_hash: DataSetHash,
    pub registry_hash: RegistryHash,
    pub random_system_version: RandomSystemVersion,
    pub simulation_profile: SimulationProfileId,
    pub tick: Tick,
    pub root_seed: RootSeed,
    pub committed_state_hash: StateHash,
}
```

Exact fields may evolve.

---

## 6. Magic identifier

Every file type begins with an explicit magic identifier.

Examples:

```text
NEWENGSV
NEWENGRP
NEWENGSN
```

This prevents accidental interpretation of unrelated files.

---

## 7. Canonical serialization

Authoritative serialization must be canonical.

Equivalent authoritative state must serialize identically under the same format version.

Requirements:

- stable field order;
- stable entity order;
- stable map ordering;
- explicit integer encoding;
- explicit float encoding;
- no pointer addresses;
- no padding bytes;
- no wall-clock metadata inside authoritative payload hashing;
- no randomized `HashMap` iteration.

---

## 8. Canonical entity order

Default:

```text
ascending stable numeric ID
```

for:

- countries;
- regions;
- provinces;
- facilities;
- formations;
- pipelines;
- contracts where persisted;
- registry entries.

---

## 9. Floating-point encoding

`f64` values are serialized through their IEEE-754 bit representation.

Conceptually:

```rust
value.to_bits()
```

Canonical save/hash logic must reject NaN authoritative state before serialization.

---

## 10. NaN handling

Authoritative state may not contain NaN.

Therefore no NaN canonicalization scheme is required for valid state.

If NaN is encountered:

```text
save fails
strict validation fails
diagnostic identifies variable/entity
```

---

## 11. Endianness

Persistence format defines byte order explicitly.

Recommended:

```text
little-endian
```

Do not use host-native byte order.

---

## 12. Integer encoding

IDs and fixed-width fields should use explicit-width integers.

Examples:

```text
u16
u32
u64
i64
```

Avoid platform-width `usize` in persistent schemas.

---

## 13. Strings

Persistent identifiers use UTF-8.

Stable keys should be normalized according to one declared project policy.

Initial recommendation:

```text
exact UTF-8 bytes from checked-in data
```

Avoid locale-sensitive transformations.

---

## 14. Schema serialization technology

The architecture does not require one serialization crate yet.

Candidates may include:

- custom binary format;
- postcard;
- bincode with tightly controlled version semantics;
- rkyv for selected snapshots;
- MessagePack/CBOR-like formats;
- protobuf/FlatBuffers/Cap'n Proto where migration/tooling benefits justify them.

The final selection must satisfy canonical encoding, versioning, safety, and deterministic hashing.

---

## 15. Do not equate serde derives with compatibility design

`serde` may be used.

But:

```rust
#[derive(Serialize, Deserialize)]
```

does not by itself define a durable save protocol.

Schema versioning and migration remain explicit responsibilities.

---

## 16. State hash algorithm

Authoritative state hashes use:

```text
BLAKE3
```

unless later changed by ADR.

Conceptual type:

```rust
pub struct StateHash(pub [u8; 32]);
```

---

## 17. Hash canonical serialization, not memory

State hash is computed from canonical authoritative serialization.

Forbidden:

```text
hash(&world_state as raw bytes)
```

Preferred:

```text
canonical writer
 -> BLAKE3
```

---

## 18. Hash scope

State hash includes all authoritative state needed to determine future simulation.

Examples:

- subsystem state;
- active delayed events;
- active pipelines;
- rolling equation memory;
- expectation state;
- registry runtime mapping if it affects behavior;
- sequential RNG state if present;
- authoritative AI state.

---

## 19. Hash exclusions

Do not hash:

- UI state;
- wall-clock save time;
- file path;
- diagnostics;
- profiling counters;
- logging state;
- caches that can be deterministically rebuilt;
- compressed representation artifacts.

---

## 20. Per-subsystem hashes

The engine should also support subsystem hashes.

Conceptually:

```rust
pub struct WorldStateHash {
    pub world: StateHash,
    pub demographics: StateHash,
    pub economy: StateHash,
    pub energy: StateHash,
    pub agriculture: StateHash,
    pub governance: StateHash,
    // ...
}
```

These help locate replay divergence.

---

## 21. Hierarchical hashing

Recommended:

```text
Subsystem state
 -> subsystem hash

ordered subsystem hashes
 + kernel authoritative state
 -> world hash
```

This improves diagnostics.

---

## 22. Hash every committed tick

In deterministic validation mode:

```text
state hash generated after every commit
```

Normal gameplay may also generate every-tick hashes if performance is acceptable.

Given BLAKE3 speed and compact state encoding, every-tick hashing is the preferred target.

---

## 23. Replay header

Conceptual:

```rust
pub struct ReplayHeader {
    pub replay_format: ReplayFormatVersion,
    pub engine_version: EngineVersion,
    pub architecture_version: ArchitectureVersion,
    pub model_set_hash: ModelSetHash,
    pub data_set_hash: DataSetHash,
    pub registry_hash: RegistryHash,
    pub random_system_version: RandomSystemVersion,
    pub simulation_profile: SimulationProfileId,
    pub root_seed: RootSeed,
    pub initial_snapshot_hash: StateHash,
}
```

---

## 24. Replay event stream

Replay records only authoritative external inputs/actions that cannot be regenerated from state.

Examples:

- player commands;
- accepted AI strategic decisions if not deterministically regenerated;
- scenario interventions;
- external imported decisions;
- branch-seed changes if supported;
- forced test outcomes;
- runtime admin/debug actions that affect state.

---

## 25. Replay does not record derived world results

Do not store:

```text
GDP result
energy price result
casualty result
```

as replay authority if they are generated by the simulation.

They should be regenerated and verified by hashes.

---

## 26. Tick replay record

Conceptually:

```rust
pub struct ReplayTickRecord {
    pub tick: Tick,
    pub actions: Vec<AuthoritativeAction>,
    pub exogenous_inputs: Vec<ExogenousInput>,
    pub expected_state_hash: StateHash,
}
```

Optional:

- subsystem hashes;
- solver summaries;
- notable event IDs.

---

## 27. Replay verification

Replay process:

```text
load initial snapshot
verify initial hash
apply tick inputs
simulate tick
compute committed hash
compare expected hash
repeat
```

Mismatch stops strict replay.

---

## 28. Divergence report

On mismatch, emit:

```text
first divergent tick
expected world hash
actual world hash
subsystem hashes
model/data versions
solver status
recent authoritative inputs
```

If subsystem hashes are stored, identify the first differing subsystem.

---

## 29. Optional deep divergence mode

Developer replay may compare:

- variable blocks;
- entity ranges;
- contract snapshots;
- solver outputs.

This is diagnostic tooling, not base replay format requirement.

---

## 30. Checkpoints

Long replays should not require simulation from tick zero.

Periodic checkpoints may store complete snapshots.

Example policy:

```text
every 12 monthly ticks
```

or:

```text
every 60 ticks
```

depending on file size/performance.

---

## 31. Checkpoint interval is non-authoritative

Changing checkpoint frequency does not change world outcomes.

It affects only storage and replay-seek performance.

---

## 32. Replay seek

To seek to tick `T`:

```text
load nearest checkpoint <= T
verify checkpoint hash
replay actions from checkpoint to T
```

---

## 33. Snapshot format

Snapshots are complete authoritative committed state.

They may be embedded in saves/replays or stored separately.

---

## 34. Initial snapshot

Every replay references or contains an initial snapshot.

A replay cannot depend on unspecified ambient world state.

---

## 35. Baseline-vs-shock experiment snapshots

Experiment harness should support:

```text
one shared initial snapshot
```

then fork:

```text
baseline action stream
shock action stream
```

This guarantees identical starting state.

---

## 36. Snapshot cloning

Experiment branching may use:

- serialized snapshot copy;
- copy-on-write state;
- in-memory clone;
- checkpoint restore.

Optimization may vary.

Semantics must remain identical.

---

## 37. Save consistency boundary

A save is taken only from a committed world state unless explicitly marked as a developer/debug mid-tick snapshot.

Normal saves occur:

```text
after CommitAuthoritativeState
after hash generation
```

---

## 38. No normal mid-solver saves

Do not save ordinary gameplay state while a solver group is partway through iteration.

This would complicate compatibility and replay substantially.

---

## 39. Crash-safe save writing

Save process:

```text
write temporary file
flush
validate checksum/hash
atomic rename/replace
```

where supported.

Avoid overwriting the only good save in place.

---

## 40. File checksum

The file container should have an integrity checksum/hash separate from committed-state hash.

Why:

```text
state hash validates semantic authoritative state
file checksum validates bytes/container integrity
```

---

## 41. Chunked file format

Recommended direction:

```text
Header
Registry/metadata chunk
Kernel state chunk
Subsystem chunks
Delayed-state chunk
Optional diagnostics chunk
Footer/integrity data
```

Chunking supports:

- partial migration;
- selective inspection;
- future compression;
- corruption localization.

---

## 42. Chunk IDs

Stable chunk identifiers.

Example:

```text
KERN
DEMO
ECON
ENER
AGRI
GOVR
INFR
MILR
PIPE
EVNT
```

Exact identifiers are implementation detail.

---

## 43. Unknown optional chunks

Reader may skip unknown chunks only if schema marks them non-authoritative/optional.

Unknown authoritative chunks require rejection unless migration support exists.

---

## 44. Compression

Compression is allowed around canonical payloads.

Hashing semantics:

```text
canonical uncompressed authoritative data
 -> state hash

compressed file bytes
 -> file integrity hash
```

Compression algorithm does not affect state hash.

---

## 45. Compression algorithm can change independently

Changing compression should not require a model-set version change.

It may require save-container format revision if file metadata changes.

---

## 46. Model-set hash

ADR-003 established a model-set hash.

Replay/save records it.

Includes semantic identity of:

- equation registry;
- variable registry;
- parameter registry;
- solver policy versions;
- contract schemas;
- active model implementations.

---

## 47. Data-set hash

Persistent files record authoritative simulation data identity.

Examples:

- country data;
- resource data;
- infrastructure baseline;
- technology catalog;
- map/province registry;
- calibration datasets used to build initial state.

The exact hashing scope must be deterministic.

---

## 48. Registry hash

Runtime mapping of stable external keys to numeric IDs should have a canonical hash.

This detects ID mapping mismatches.

---

## 49. Engine version

Record application/build identity.

Recommended:

```text
semantic version
+
git commit SHA
```

for development builds.

---

## 50. Architecture version

Architecture baseline/ADR compatibility can be recorded separately.

It is descriptive and helps diagnostics.

---

## 51. Compatibility decision matrix

On load/replay compare:

```text
save format
engine version
model set hash
data set hash
registry hash
random system version
```

Each mismatch has explicit policy.

---

## 52. Format mismatch

If reader does not support file format version:

```text
reject
```

unless a migration path exists.

---

## 53. Engine version mismatch

Engine binary version mismatch alone is not necessarily fatal.

If:

- format supported;
- model-set hash compatible;
- data compatible;

load may proceed.

---

## 54. Model-set mismatch

Default:

```text
reject exact replay verification
```

A save migration may be allowed if explicit migration transforms state.

Do not silently load and claim deterministic continuity.

---

## 55. Data-set mismatch

If authoritative baseline data changed:

```text
reject
```

unless save is self-contained enough that changed external data is not required or a migration explicitly handles it.

---

## 56. Registry mismatch

Reject unless numeric-ID mapping can be migrated safely by stable external keys.

---

## 57. PRNG/random-system mismatch

Exact replay requires compatible random algorithm/version.

If incompatible:

```text
reject exact replay
```

Old-save continuation may require compatibility implementation.

---

## 58. Migration philosophy

Migrations are explicit transformations:

```text
SchemaVersion N
 -> SchemaVersion N+1
```

They are:

- deterministic;
- tested;
- logged;
- versioned.

---

## 59. No silent best-effort migration

If a required field cannot be mapped reliably:

```text
migration fails
```

Do not invent authoritative values without a declared migration rule.

---

## 60. Migration chain

Reader may apply:

```text
v3 -> v4 -> v5
```

rather than requiring every old version to migrate directly to latest.

---

## 61. Migration tests

Each migration requires fixtures:

```text
old save
 -> migrate
 -> validate
 -> expected canonical state/hash
```

---

## 62. Semantic migrations

Some changes require model-state transformation.

Example:

Old:

```text
single energy_inventory
```

New:

```text
oil_inventory
gas_inventory
coal_inventory
```

A migration is valid only if a defensible mapping exists.

Otherwise old saves may be declared incompatible.

---

## 63. Compatibility is not absolute

The project does not promise indefinite compatibility with every development build.

Recommended policy:

```text
development:
    limited compatibility

stable releases:
    stronger migration support
```

---

## 64. Save provenance

Save metadata should record:

- scenario name/key;
- world creation version;
- initial data set;
- model profile;
- root seed;
- current tick;
- branch lineage if supported.

---

## 65. Branch lineage

A save created from another save may optionally record:

```text
parent save hash
branch tick
```

Useful for experiments/debugging.

---

## 66. Replay lineage

A branch replay may record:

```text
parent replay hash
fork tick
```

---

## 67. Content-addressable snapshots

Snapshots may optionally be named/stored by state hash.

Example:

```text
snapshot-<hash>.bin
```

This helps deduplication in experiment tooling.

---

## 68. Save file naming is non-authoritative

Human filenames are not part of simulation identity.

---

## 69. Scenario reproducibility bundle

For research runs, export a bundle containing:

```text
initial snapshot
scenario inputs
model/data hashes
seed
replay stream
result summary
```

This supports reproducible experiments.

---

## 70. Replay hash

The replay stream itself should have a content hash.

This provides identity for a run's action/input history.

---

## 71. Run identity

Conceptually:

```rust
pub struct RunIdentity {
    pub initial_state_hash: StateHash,
    pub replay_stream_hash: ReplayHash,
    pub model_set_hash: ModelSetHash,
}
```

---

## 72. World identity vs run identity

Two runs can share initial world state but diverge through actions.

Therefore:

```text
world snapshot identity != run identity
```

---

## 73. Snapshot validation

On load:

1. validate container integrity;
2. validate format;
3. validate schema/chunks;
4. validate model/data compatibility;
5. deserialize;
6. validate domain invariants;
7. recompute canonical state hash;
8. compare stored hash.

Only then accept state.

---

## 74. Domain validation after load

Examples:

- nonnegative stocks;
- valid IDs;
- pipeline owners exist;
- delayed-event targets exist;
- queue lengths valid;
- shares/rates in bounds;
- registry references valid.

---

## 75. Corrupt file behavior

Reject cleanly.

Do not partially continue simulation from uncertain authoritative state.

---

## 76. Fuzzing

Persistence readers should be fuzz-tested.

Targets:

- malformed lengths;
- integer overflow;
- unknown chunks;
- invalid IDs;
- truncated data;
- duplicate chunks;
- corrupted compressed blocks.

---

## 77. Resource limits

Reader must defend against malicious/corrupt allocations.

Example:

```text
file claims vector length = 2^63
```

Reject before allocation.

---

## 78. Untrusted save policy

Treat save files as untrusted input.

Even single-player files can be corrupt or modified.

---

## 79. No code execution in save format

Save/replay files contain data only.

No embedded arbitrary scripts/macros that execute during deserialization.

---

## 80. External scenario references

If a save depends on external scenario content, record stable content hashes.

Prefer self-contained authoritative state where practical.

---

## 81. Mod compatibility

Future mods may change:

- equations;
- registries;
- data;
- unit definitions.

Save records mod/model identity hashes.

A mismatched mod set should not silently load.

---

## 82. Mod list fingerprint

Potential:

```text
ordered mod ID
version
content hash
```

This is future-facing but compatible with the design.

---

## 83. Deterministic serialization tests

Same authoritative state serialized twice must produce identical canonical payload bytes.

---

## 84. Round-trip test

```text
state
 -> serialize
 -> deserialize
 -> state
```

must preserve state hash.

---

## 85. Save/load continuation test

Run:

```text
0 -> 100 ticks
```

Compare with:

```text
0 -> 50
save
load
50 -> 100
```

Final and per-tick hashes after load must match uninterrupted run.

---

## 86. Replay reproduction test

Run once, create replay.

Then replay from initial snapshot.

Every recorded tick hash must match.

---

## 87. Parallel replay test

Replay using supported thread counts.

Hashes must match within ADR-001's reproducibility target.

---

## 88. Checkpoint seek test

Replay:

```text
0 -> 200
```

Then seek from checkpoint:

```text
120 -> 200
```

Hashes from 120 onward must match.

---

## 89. Corruption test

Flip bytes in save/replay.

Reader must reject due to:

- checksum;
- parse error;
- hash mismatch;
- invariant failure.

---

## 90. Migration regression test

Old fixture migrates to expected latest-state hash.

---

## 91. Hash stability test vectors

Canonical state fixtures should have known expected BLAKE3 hashes.

This protects serialization/hash semantics from accidental drift.

---

## 92. Subsystem hash debugging

If world hash diverges:

```text
compare subsystem hashes
```

Then deeper tools may compare subsystem blocks.

---

## 93. Binary diff tooling

`sim-tools` should eventually support:

```powershell
cargo run -p sim-tools -- save inspect file.sav
cargo run -p sim-tools -- save verify file.sav
cargo run -p sim-tools -- replay verify file.rep
cargo run -p sim-tools -- snapshot hash file.snap
cargo run -p sim-tools -- state diff a.snap b.snap
```

---

## 94. Save inspection

Inspection should show:

- versions;
- hashes;
- tick/date;
- profile;
- seed;
- subsystem chunk sizes;
- active pipeline/event counts.

---

## 95. Replay inspection

Show:

- initial snapshot hash;
- seed;
- tick range;
- action counts;
- checkpoint list;
- expected final hash.

---

## 96. Diff tooling

State diff should identify:

```text
subsystem
variable
entity
expected
actual
```

where schema metadata permits.

---

## 97. Causal debugging integration

A replay divergence tool may combine:

- first divergent tick;
- state diff;
- equation trace;
- solver diagnostics.

This is a long-term target.

---

## 98. Snapshot frequency for tests

CI deterministic tests may snapshot every tick or selected ticks.

Production saves need not store every snapshot.

---

## 99. Replay storage efficiency

Replay should be much smaller than full snapshots because it stores mostly external actions plus hashes/checkpoints.

---

## 100. Action ordering

Multiple authoritative actions in the same tick must have canonical ordering.

Example:

```text
phase
actor ID
action sequence
```

This ordering is recorded.

---

## 101. Action IDs

Authoritative actions should carry stable IDs where needed.

This aids replay diagnostics.

---

## 102. External-input ordering

Scenario shocks/events applied in one tick must also be canonically ordered.

---

## 103. Replay and random draws

Per ADR-006, individual random draws are normally regenerated.

Replay stores:

- root seed;
- random-system version.

Optional debug draw logs are not authoritative replay input.

---

## 104. Replay and AI

If strategic AI is fully deterministic from state, its actions may be regenerated.

If AI behavior depends on external/non-authoritative components, accepted AI actions must be recorded in replay.

---

## 105. LLM/remote AI prohibition in authoritative replay path

If future AI features call a remote/nondeterministic model, their raw output cannot directly be assumed reproducible.

Any accepted action from such a system must be materialized as authoritative replay input.

---

## 106. Hot reload implications

Changing model code/data during a running authoritative simulation invalidates exact replay unless version transition is explicitly supported.

Development hot reload should mark the run non-verifiable or fork a new model identity.

---

## 107. Debug mutation

Developer console commands that change authoritative state must become replay events.

Example:

```text
set energy.capacity country=X value=Y
```

If not recorded, deterministic replay is invalid.

---

## 108. Cheats/modifiers

Gameplay cheats that alter authoritative state are treated as actions and recorded.

---

## 109. Save metadata vs authoritative hash

Human-readable metadata may include:

- save name;
- description;
- wall-clock timestamp;
- screenshot reference.

This metadata is excluded from authoritative state hash.

---

## 110. Autosave

Autosave occurs only at safe commit boundaries.

---

## 111. Rolling autosaves

Application may retain:

```text
N most recent autosaves
```

This is product behavior, not simulation semantics.

---

## 112. Save compression and large worlds

Large dense states may require chunk-level compression.

Potential codecs:

- zstd;
- lz4.

Exact choice is implementation detail.

---

## 113. Incremental saves

Future optimization may store:

```text
base snapshot + deltas
```

but v0.1 should prefer complete self-contained saves for reliability.

---

## 114. Delta snapshots

Experiment systems may use in-memory deltas/copy-on-write.

Persistent gameplay saves remain complete initially.

---

## 115. State serialization ownership

Each subsystem owns serialization/migration of its authoritative state schema through defined interfaces.

The kernel owns container assembly.

---

## 116. Subsystem schema version

Subsystem chunks may carry local schema version.

Example:

```rust
pub struct SubsystemSchemaVersion(pub u16);
```

This can simplify migrations.

---

## 117. Kernel-state schema

Kernel authoritative state also has schema version.

Includes:

- tick/calendar;
- event queues;
- scheduling state that affects future;
- root seed metadata;
- replay lineage.

---

## 118. Contract snapshots are generally not persisted

Contracts can usually be regenerated from committed subsystem state after load.

Persist only if a specific mid-boundary state requires them, which normal saves do not.

---

## 119. Solver intermediate state is not persisted

Normal saves occur after commit.

No need to persist:

- current iteration number;
- residuals;
- candidate snapshots.

---

## 120. Derived history persistence

Only history required by future equations is authoritative.

Analytics history can be stored separately.

---

## 121. Rolling-memory persistence

If a 12-month moving average requires previous values, the ring buffer/window state is persisted.

---

## 122. Pipeline persistence

All active pipeline information required for future progress is persisted.

---

## 123. Event queue persistence

All future scheduled authoritative events are persisted.

---

## 124. Queue/backlog persistence

Backlogs are authoritative and persisted.

---

## 125. Perceived-state persistence

If future ADRs introduce perceived/intelligence state that affects decisions, that state is authoritative for the relevant actor and must be persisted.

---

## 126. Hashing perceived state

If perceived state influences future authoritative decisions, include it in state hash.

---

## 127. Non-authoritative UI fog

Pure presentation-only visibility masks are excluded.

---

## 128. Save determinism

Saving itself must not alter simulation state.

---

## 129. Load determinism

Loading must not consume random draws or advance time.

---

## 130. Serialization side-effect prohibition

Serialization functions are pure with respect to authoritative state.

---

## 131. Migration side effects

Migration may transform state but must not:

- call external services;
- use wall clock;
- use nondeterministic randomness.

---

## 132. Stable external keys in migration

Where numeric IDs changed, migrate using stable external keys.

Example:

```text
"USA"
"energy.oil"
"sector.manufacturing"
```

---

## 133. Missing data during migration

If required stable key no longer exists:

```text
migration fails
```

unless a declared mapping exists.

---

## 134. Save compatibility report

On load failure, present clear reasons:

```text
unsupported save format
model mismatch
data mismatch
registry mismatch
corrupt file
migration unavailable
```

---

## 135. Strict replay mode

Strict replay fails immediately on:

- hash mismatch;
- incompatible model;
- invalid action stream;
- missing checkpoint data;
- corrupted snapshot.

---

## 136. Exploratory replay mode

Developer tooling may optionally continue after mismatch for diagnosis.

It must clearly mark results non-authoritative/non-verifying.

---

## 137. Experiment result manifests

Research experiments should store:

```text
run ID
initial hash
scenario hash
model hash
seed
final hash
key metrics
```

---

## 138. Scenario hash

Scenario inputs/configuration should have canonical content hash.

This lets two experiments verify they used identical intervention definitions.

---

## 139. Configuration hash

Authoritative simulation configuration also receives a canonical hash.

Includes:

- solver profile;
- fidelity profile;
- cadence profile;
- active model implementations.

---

## 140. Full run fingerprint

Conceptually:

```text
initial_state_hash
model_set_hash
data_set_hash
scenario_hash
config_hash
root_seed
replay_stream_hash
```

This uniquely identifies the deterministic run context.

---

## 141. Golden replays

The repository should maintain small golden replays.

Examples:

```text
minimal-energy-shock
fertilizer-delay
military-procurement
```

CI replays them and verifies tick hashes.

---

## 142. Golden replay update discipline

If a legitimate model change alters hashes:

- update model version;
- explain change;
- regenerate fixtures intentionally.

Do not casually overwrite golden hashes.

---

## 143. Build reproducibility

Cargo dependency lockfile should be committed.

For stable release builds, build metadata should record dependency lock/content where practical.

---

## 144. Rust compiler version

Authoritative builds should use a pinned toolchain via:

```text
rust-toolchain.toml
```

as anticipated in ADR-001.

---

## 145. Cross-platform compatibility

v0.1 persistence files should be platform-independent in encoding.

Exact cross-platform floating-point replay remains subject to ADR-001's reproducibility target.

A save created on Windows should be readable on Linux if model/data compatibility is satisfied.

---

## 146. Cross-platform hash caution

If different platforms produce different authoritative floating calculations, state hashes may diverge even though file format is portable.

This is a simulation reproducibility issue, not a serialization issue.

---

## 147. Future cross-platform determinism

If required later, stronger numerical constraints/fixed-point algorithms may be introduced.

ADR-007 does not mandate them.

---

## 148. Cloud/save sync

Cloud synchronization may copy save files.

It does not change format semantics.

Conflict resolution should not merge binary authoritative state.

Choose one save/version explicitly.

---

## 149. No automatic semantic merge

Two divergent saves cannot be merged generically.

They represent different worlds.

---

## 150. Security/privacy

Save files may include detailed scenario/world state.

Encryption is optional product functionality and separate from deterministic serialization.

If encrypted, decrypt before canonical semantic validation.

---

## 151. Signing

Future multiplayer/competitive modes may sign replay/save metadata.

Not required for v0.1.

---

## 152. Cargo crate direction

Possible crates/modules:

```text
sim-persistence/
    save container
    replay container
    snapshot encoding
    migrations
    integrity validation

sim-hash/
    canonical state hashing
```

These may initially be modules if separate crates are premature.

---

## 153. Dependency direction

Persistence depends on stable schema/types.

Subsystem implementation should not depend on persistence container logic.

Preferred:

```text
sim-persistence
    -> sim-state schemas
    -> sim-types
```

not:

```text
subsystem-energy
    -> save-file container implementation
```

---

## 154. Serialization traits

Conceptual:

```rust
pub trait CanonicalEncode {
    fn encode_canonical(
        &self,
        writer: &mut dyn CanonicalWriter,
    ) -> Result<(), EncodeError>;
}
```

Potential decode/migrate traits may be separate.

---

## 155. Avoid blanket generic serialization in hot architecture

Explicit canonical encoders for authoritative schema may be preferable to magical recursive serialization where ordering/compatibility is critical.

---

## 156. Generated schema code

If schema tooling is adopted later, generated code is acceptable if:

- checked/versioned;
- deterministic;
- compatible with migrations;
- inspectable.

---

## 157. Acceptance criteria

ADR-007 is implemented when:

1. `StateHash` using BLAKE3 exists;
2. canonical serialization rules exist in code;
3. save/replay/snapshot format versions exist;
4. complete committed state can be saved and loaded;
5. delayed events/pipelines/expectations survive load;
6. every-tick replay hash verification works;
7. per-subsystem hashes exist;
8. save/load continuation matches uninterrupted run;
9. replay reproduction matches every tick hash;
10. corrupt saves are rejected;
11. checkpoint seek reproduces hashes;
12. model/data/registry mismatch checks exist;
13. at least one schema migration test exists;
14. `sim-tools` can inspect/verify saves and replays.

---

## 158. Initial implementation sequence

### Step 1

Implement:

```text
StateHash
ModelSetHash
DataSetHash
RegistryHash
ScenarioHash
ConfigHash
```

### Step 2

Implement canonical primitive writer:

```text
u8/u16/u32/u64
i64
f64 bits
byte arrays
UTF-8 strings
```

### Step 3

Canonical encode minimal vertical-slice state.

### Step 4

Compute per-subsystem and world BLAKE3 hashes.

### Step 5

Implement snapshot container.

### Step 6

Implement save container.

### Step 7

Implement replay stream with per-tick expected hash.

### Step 8

Add save/load continuation test.

### Step 9

Add replay verification test.

### Step 10

Add checkpoint support.

### Step 11

Add migration framework.

---

## 159. Consequences

### Positive

- deterministic replay becomes provable;
- save/load cannot silently alter world state;
- experiment runs are reproducible;
- divergence can be localized by subsystem;
- file corruption is detectable;
- model/data mismatches are explicit;
- schema evolution is managed;
- long replays can seek through checkpoints.

### Costs

- explicit schema/migration work;
- canonical encoders require discipline;
- save compatibility creates maintenance burden;
- per-tick hashing has some performance cost;
- development builds may intentionally break compatibility.

These costs are accepted.

---

## 160. Rejected alternatives

### Serialize Rust structs directly and hope for compatibility

Rejected because Rust layout/type evolution is not a persistence protocol.

### Replay by storing every derived state value

Rejected because it is large and hides determinism bugs.

### No state hashes

Rejected because divergence would be difficult to detect and localize.

### Load mismatched models silently

Rejected because resulting simulation would not be reproducible or trustworthy.

### Save mid-solver as normal behavior

Rejected because it complicates semantics substantially.

### Hash compressed file bytes as world identity

Rejected because compression representation is not authoritative state semantics.

---

## 161. Open implementation questions

Still open:

- exact serialization/container technology;
- compression codec;
- checkpoint interval defaults;
- exact migration support policy for development vs releases;
- exact subsystem chunk boundaries;
- whether canonical encoding is custom or schema-library-backed;
- whether content-addressable snapshot storage is adopted early;
- exact mod fingerprint format.

These do not change the core decision.

---

# Decision summary

New Engine will use explicit versioned save, snapshot, and replay protocols.

Persistence is based on:

- canonical authoritative serialization;
- BLAKE3 committed-state hashes;
- per-subsystem hashes;
- model/data/registry/config fingerprints;
- recorded PRNG identity and root seed;
- complete delayed-state persistence;
- action-stream replays with per-tick verification;
- periodic checkpoints;
- explicit deterministic migrations;
- strict compatibility and corruption checks.

A replay is valid only if it regenerates the expected committed world-state hash at each verification point.

This ADR establishes the persistence and reproducibility foundation for New Engine.
