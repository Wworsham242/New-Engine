param(
    [string]$RepoRoot = (Get-Location).Path,
    [switch]$Force
)

$ErrorActionPreference = "Stop"

function Write-Utf8NoBom {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Content
    )
    $parent = Split-Path -Parent $Path
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    $utf8 = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText($Path, $Content, $utf8)
}

$RepoRoot = [System.IO.Path]::GetFullPath($RepoRoot)
if (-not (Test-Path (Join-Path $RepoRoot ".git"))) {
    throw "RepoRoot does not look like the New Engine Git repository: $RepoRoot"
}

$rootCargo = Join-Path $RepoRoot "Cargo.toml"
if ((Test-Path $rootCargo) -and -not $Force) {
    throw "Cargo.toml already exists. This is a one-time Phase 001 bootstrap. Re-run with -Force only if you intentionally want to overwrite the generated scaffold."
}

Write-Host "Bootstrapping New Engine Phase 001 in: $RepoRoot"

# ---------------------------------------------------------------------------
# Root workspace
# ---------------------------------------------------------------------------

Write-Utf8NoBom $rootCargo @'
[workspace]
resolver = "3"
members = [
    "crates/sim-types",
    "crates/sim-storage",
    "crates/sim-units",
    "crates/sim-equations",
    "crates/sim-contracts",
    "crates/sim-state",
    "crates/sim-kernel",
    "crates/sim-random",
    "crates/sim-tools",
    "crates/subsystem-demographics",
    "crates/subsystem-energy",
    "crates/subsystem-economy",
    "crates/subsystem-governance",
]

[workspace.package]
version = "0.1.0"
edition = "2024"

[workspace.dependencies]
blake3 = "1"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "rust-toolchain.toml") @'
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy"]
profile = "minimal"
'@

# ---------------------------------------------------------------------------
# sim-types
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-types\Cargo.toml") @'
[package]
name = "sim-types"
version.workspace = true
edition.workspace = true

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-types\src\lib.rs") @'
#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Tick(pub u64);

impl Tick {
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CountryId(pub u32);

impl CountryId {
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u16)]
pub enum SubsystemId {
    Demographics = 1,
    Economy = 2,
    Energy = 3,
    Governance = 4,
    Military = 5,
    Infrastructure = 6,
    Agriculture = 7,
    Environment = 8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceTag {
    OriginalDesign,
    IfsObservedArchitecture,
    PublicLiterature,
    DatasetDerived,
    MilitaryReferenceModel,
    PerformanceSimplification,
    GameplayAbstraction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cadence {
    Monthly,
    Quarterly,
    Annual,
    InternalSubstep,
    OnEvent,
}
'@

# ---------------------------------------------------------------------------
# sim-units
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-units\Cargo.toml") @'
[package]
name = "sim-units"
version.workspace = true
edition.workspace = true

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-units\src\lib.rs") @'
#![forbid(unsafe_code)]

macro_rules! scalar_unit {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
        pub struct $name(pub f64);

        impl $name {
            pub fn nonnegative(self) -> Self {
                Self(self.0.max(0.0))
            }

            pub fn is_finite(self) -> bool {
                self.0.is_finite()
            }
        }
    };
}

scalar_unit!(RealGdp);
scalar_unit!(Currency);
scalar_unit!(EnergyQuantity);
scalar_unit!(PriceIndex);
scalar_unit!(Rate);
scalar_unit!(Population);
'@

# ---------------------------------------------------------------------------
# sim-storage
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-storage\Cargo.toml") @'
[package]
name = "sim-storage"
version.workspace = true
edition.workspace = true

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-storage\src\lib.rs") @'
#![forbid(unsafe_code)]

#[derive(Clone, Debug, PartialEq)]
pub struct Dense1<T> {
    values: Vec<T>,
}

impl<T: Clone> Dense1<T> {
    pub fn new(len: usize, value: T) -> Self {
        Self {
            values: vec![value; len],
        }
    }
}

impl<T> Dense1<T> {
    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.values.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.values.get_mut(index)
    }

    pub fn as_slice(&self) -> &[T] {
        &self.values
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Dense2<T> {
    rows: usize,
    cols: usize,
    values: Vec<T>,
}

impl<T: Clone> Dense2<T> {
    pub fn new(rows: usize, cols: usize, value: T) -> Self {
        Self {
            rows,
            cols,
            values: vec![value; rows * cols],
        }
    }
}

impl<T> Dense2<T> {
    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn get(&self, row: usize, col: usize) -> Option<&T> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        self.values.get(row * self.cols + col)
    }

    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut T> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        self.values.get_mut(row * self.cols + col)
    }

    pub fn canonical_values(&self) -> &[T] {
        &self.values
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct StableHandle(pub u32);

#[derive(Clone, Debug, Default)]
pub struct StableArena<T> {
    values: Vec<T>,
}

impl<T> StableArena<T> {
    pub fn insert(&mut self, value: T) -> StableHandle {
        let handle = StableHandle(
            u32::try_from(self.values.len())
                .expect("stable arena exceeded u32 handle capacity"),
        );
        self.values.push(value);
        handle
    }

    pub fn get(&self, handle: StableHandle) -> Option<&T> {
        self.values.get(handle.0 as usize)
    }

    pub fn get_mut(&mut self, handle: StableHandle) -> Option<&mut T> {
        self.values.get_mut(handle.0 as usize)
    }

    pub fn iter_canonical(&self) -> impl Iterator<Item = (StableHandle, &T)> {
        self.values
            .iter()
            .enumerate()
            .map(|(index, value)| (StableHandle(index as u32), value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dense2_is_row_major_and_deterministic() {
        let mut store = Dense2::new(2, 3, 0_u32);
        *store.get_mut(1, 2).unwrap() = 9;
        assert_eq!(store.canonical_values(), &[0, 0, 0, 0, 0, 9]);
    }

    #[test]
    fn stable_arena_iterates_in_handle_order() {
        let mut arena = StableArena::default();
        let a = arena.insert("a");
        let b = arena.insert("b");
        assert_eq!(a, StableHandle(0));
        assert_eq!(b, StableHandle(1));
        let values: Vec<_> = arena.iter_canonical().map(|(_, v)| *v).collect();
        assert_eq!(values, vec!["a", "b"]);
    }
}
'@

# ---------------------------------------------------------------------------
# sim-equations
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-equations\Cargo.toml") @'
[package]
name = "sim-equations"
version.workspace = true
edition.workspace = true

[dependencies]
sim-types = { path = "../sim-types" }

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-equations\src\lib.rs") @'
#![forbid(unsafe_code)]

use sim_types::{Cadence, ProvenanceTag, SubsystemId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EquationId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EquationClass {
    AccountingIdentity,
    StockFlow,
    Behavioral,
    Capacity,
    Allocation,
    MarketAdjustment,
    Lag,
    Constraint,
    Aggregation,
    SolverComponent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalReference {
    CurrentCommitted,
    CurrentIteration,
    PreviousTick,
    RollingAverage,
    PipelineState,
}

#[derive(Clone, Debug)]
pub struct EquationMetadata {
    pub id: EquationId,
    pub key: &'static str,
    pub owner: SubsystemId,
    pub class: EquationClass,
    pub cadence: Cadence,
    pub temporal_reference: TemporalReference,
    pub provenance: &'static [ProvenanceTag],
}
'@

# ---------------------------------------------------------------------------
# sim-contracts
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-contracts\Cargo.toml") @'
[package]
name = "sim-contracts"
version.workspace = true
edition.workspace = true

[dependencies]
sim-units = { path = "../sim-units" }

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-contracts\src\lib.rs") @'
#![forbid(unsafe_code)]

use sim_units::{Currency, EnergyQuantity, Population, PriceIndex, Rate, RealGdp};

#[derive(Clone, Copy, Debug, Default)]
pub struct EconomyToEnergy {
    pub gdp: RealGdp,
    pub investment: Currency,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EnergyToEconomy {
    pub production: EnergyQuantity,
    pub demand: EnergyQuantity,
    pub price_index: PriceIndex,
    pub shortage_fraction: Rate,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EconomyToGovernance {
    pub taxable_output: RealGdp,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DemographicsOutputs {
    pub population: Population,
}
'@

# ---------------------------------------------------------------------------
# sim-state
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-state\Cargo.toml") @'
[package]
name = "sim-state"
version.workspace = true
edition.workspace = true

[dependencies]
blake3.workspace = true
sim-types = { path = "../sim-types" }
sim-units = { path = "../sim-units" }

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-state\src\lib.rs") @'
#![forbid(unsafe_code)]

use sim_types::Tick;
use sim_units::{Currency, EnergyQuantity, Population, PriceIndex, Rate, RealGdp};

#[derive(Clone, Debug)]
pub struct EnergyState {
    pub capacity: EnergyQuantity,
    pub production: EnergyQuantity,
    pub demand: EnergyQuantity,
    pub price_index: PriceIndex,
    pub shortage_fraction: Rate,
}

#[derive(Clone, Debug)]
pub struct EconomyState {
    pub gdp: RealGdp,
    pub potential_gdp: RealGdp,
    pub investment: Currency,
}

#[derive(Clone, Debug)]
pub struct GovernanceState {
    pub tax_rate: Rate,
    pub revenue: Currency,
    pub spending: Currency,
}

#[derive(Clone, Debug)]
pub struct DemographicsState {
    pub population: Population,
}

#[derive(Clone, Debug)]
pub struct WorldState {
    pub tick: Tick,
    pub energy: EnergyState,
    pub economy: EconomyState,
    pub governance: GovernanceState,
    pub demographics: DemographicsState,
}

impl Default for WorldState {
    fn default() -> Self {
        Self {
            tick: Tick(0),
            energy: EnergyState {
                capacity: EnergyQuantity(100.0),
                production: EnergyQuantity(100.0),
                demand: EnergyQuantity(100.0),
                price_index: PriceIndex(1.0),
                shortage_fraction: Rate(0.0),
            },
            economy: EconomyState {
                gdp: RealGdp(1_000.0),
                potential_gdp: RealGdp(1_000.0),
                investment: Currency(200.0),
            },
            governance: GovernanceState {
                tax_rate: Rate(0.20),
                revenue: Currency(200.0),
                spending: Currency(220.0),
            },
            demographics: DemographicsState {
                population: Population(1_000_000.0),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateHash(pub [u8; 32]);

impl StateHash {
    pub fn to_hex(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl WorldState {
    pub fn validate(&self) -> Result<(), &'static str> {
        let finite = [
            self.energy.capacity.0,
            self.energy.production.0,
            self.energy.demand.0,
            self.energy.price_index.0,
            self.energy.shortage_fraction.0,
            self.economy.gdp.0,
            self.economy.potential_gdp.0,
            self.economy.investment.0,
            self.governance.tax_rate.0,
            self.governance.revenue.0,
            self.governance.spending.0,
            self.demographics.population.0,
        ]
        .into_iter()
        .all(f64::is_finite);

        if !finite {
            return Err("authoritative state contains a non-finite value");
        }

        if self.energy.capacity.0 < 0.0
            || self.energy.production.0 < 0.0
            || self.energy.demand.0 < 0.0
            || self.economy.gdp.0 < 0.0
            || self.demographics.population.0 < 0.0
        {
            return Err("authoritative state contains a negative physical stock/flow");
        }

        Ok(())
    }

    pub fn canonical_hash(&self) -> StateHash {
        let mut hasher = blake3::Hasher::new();

        hasher.update(&self.tick.0.to_le_bytes());

        for value in [
            self.energy.capacity.0,
            self.energy.production.0,
            self.energy.demand.0,
            self.energy.price_index.0,
            self.energy.shortage_fraction.0,
            self.economy.gdp.0,
            self.economy.potential_gdp.0,
            self.economy.investment.0,
            self.governance.tax_rate.0,
            self.governance.revenue.0,
            self.governance.spending.0,
            self.demographics.population.0,
        ] {
            hasher.update(&value.to_bits().to_le_bytes());
        }

        StateHash(*hasher.finalize().as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_state_has_equal_hash() {
        let a = WorldState::default();
        let b = a.clone();
        assert_eq!(a.canonical_hash(), b.canonical_hash());
    }

    #[test]
    fn changed_state_changes_hash() {
        let a = WorldState::default();
        let mut b = a.clone();
        b.energy.capacity.0 -= 1.0;
        assert_ne!(a.canonical_hash(), b.canonical_hash());
    }
}
'@

# ---------------------------------------------------------------------------
# sim-random
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-random\Cargo.toml") @'
[package]
name = "sim-random"
version.workspace = true
edition.workspace = true

[dependencies]
blake3.workspace = true
sim-types = { path = "../sim-types" }

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-random\src\lib.rs") @'
#![forbid(unsafe_code)]

use sim_types::{SubsystemId, Tick};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RootSeed(pub [u8; 32]);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RandomKey {
    pub subsystem: SubsystemId,
    pub tick: Tick,
    pub entity: u64,
    pub event_kind: u32,
    pub draw_index: u32,
}

impl RandomKey {
    pub fn canonical_fingerprint(self, seed: RootSeed) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new_keyed(&seed.0);
        hasher.update(b"new-engine.random-key.v0");
        hasher.update(&(self.subsystem as u16).to_le_bytes());
        hasher.update(&self.tick.0.to_le_bytes());
        hasher.update(&self.entity.to_le_bytes());
        hasher.update(&self.event_kind.to_le_bytes());
        hasher.update(&self.draw_index.to_le_bytes());
        *hasher.finalize().as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_has_same_fingerprint() {
        let seed = RootSeed([7; 32]);
        let key = RandomKey {
            subsystem: SubsystemId::Energy,
            tick: Tick(12),
            entity: 4,
            event_kind: 2,
            draw_index: 0,
        };

        assert_eq!(
            key.canonical_fingerprint(seed),
            key.canonical_fingerprint(seed)
        );
    }
}
'@

# ---------------------------------------------------------------------------
# subsystem-energy
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\subsystem-energy\Cargo.toml") @'
[package]
name = "subsystem-energy"
version.workspace = true
edition.workspace = true

[dependencies]
sim-contracts = { path = "../sim-contracts" }
sim-state = { path = "../sim-state" }
sim-units = { path = "../sim-units" }

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\subsystem-energy\src\lib.rs") @'
#![forbid(unsafe_code)]

use sim_contracts::{EconomyToEnergy, EnergyToEconomy};
use sim_state::EnergyState;
use sim_units::{EnergyQuantity, PriceIndex, Rate};

pub fn apply_capacity_shock(state: &mut EnergyState, fraction_lost: f64) {
    let bounded = fraction_lost.clamp(0.0, 1.0);
    state.capacity.0 *= 1.0 - bounded;
}

pub fn publish(state: &EnergyState) -> EnergyToEconomy {
    EnergyToEconomy {
        production: state.production,
        demand: state.demand,
        price_index: state.price_index,
        shortage_fraction: state.shortage_fraction,
    }
}

/// Phase-001 scaffold equation only.
///
/// This is deliberately simple and uncalibrated. It proves the causal and
/// solver architecture; it is not an IFs equation and is not intended to be
/// retained as the production energy model.
pub fn solve_candidate(
    current: &EnergyState,
    economy: EconomyToEnergy,
) -> EnergyState {
    let demand = (economy.gdp.0 / 10.0).max(1.0);
    let production = current.capacity.0.min(demand).max(0.0);
    let shortage = ((demand - production) / demand).clamp(0.0, 1.0);
    let price = 1.0 + 2.0 * shortage;

    EnergyState {
        capacity: current.capacity,
        production: EnergyQuantity(production),
        demand: EnergyQuantity(demand),
        price_index: PriceIndex(price),
        shortage_fraction: Rate(shortage),
    }
}

pub fn blend(
    previous: &EnergyState,
    candidate: &EnergyState,
    damping: f64,
) -> EnergyState {
    let a = damping.clamp(0.0, 1.0);
    let lerp = |old: f64, new: f64| old + a * (new - old);

    EnergyState {
        capacity: candidate.capacity,
        production: EnergyQuantity(lerp(previous.production.0, candidate.production.0)),
        demand: EnergyQuantity(lerp(previous.demand.0, candidate.demand.0)),
        price_index: PriceIndex(lerp(previous.price_index.0, candidate.price_index.0)),
        shortage_fraction: Rate(lerp(
            previous.shortage_fraction.0,
            candidate.shortage_fraction.0,
        )),
    }
}
'@

# ---------------------------------------------------------------------------
# subsystem-economy
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\subsystem-economy\Cargo.toml") @'
[package]
name = "subsystem-economy"
version.workspace = true
edition.workspace = true

[dependencies]
sim-contracts = { path = "../sim-contracts" }
sim-state = { path = "../sim-state" }
sim-units = { path = "../sim-units" }

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\subsystem-economy\src\lib.rs") @'
#![forbid(unsafe_code)]

use sim_contracts::{EconomyToEnergy, EconomyToGovernance, EnergyToEconomy};
use sim_state::EconomyState;
use sim_units::RealGdp;

pub fn publish_to_energy(state: &EconomyState) -> EconomyToEnergy {
    EconomyToEnergy {
        gdp: state.gdp,
        investment: state.investment,
    }
}

pub fn publish_to_governance(state: &EconomyState) -> EconomyToGovernance {
    EconomyToGovernance {
        taxable_output: state.gdp,
    }
}

/// Phase-001 scaffold equation only.
///
/// The coefficient is an engineering placeholder used to prove a coupled
/// Energy <-> Economy solve. It is not calibrated and is not copied from IFs.
pub fn solve_candidate(
    current: &EconomyState,
    energy: EnergyToEconomy,
) -> EconomyState {
    let availability_factor =
        (1.0 - 0.25 * energy.shortage_fraction.0).clamp(0.0, 1.0);

    EconomyState {
        gdp: RealGdp(current.potential_gdp.0 * availability_factor),
        potential_gdp: current.potential_gdp,
        investment: current.investment,
    }
}

pub fn blend(
    previous: &EconomyState,
    candidate: &EconomyState,
    damping: f64,
) -> EconomyState {
    let a = damping.clamp(0.0, 1.0);
    EconomyState {
        gdp: RealGdp(previous.gdp.0 + a * (candidate.gdp.0 - previous.gdp.0)),
        potential_gdp: candidate.potential_gdp,
        investment: candidate.investment,
    }
}
'@

# ---------------------------------------------------------------------------
# subsystem-governance
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\subsystem-governance\Cargo.toml") @'
[package]
name = "subsystem-governance"
version.workspace = true
edition.workspace = true

[dependencies]
sim-contracts = { path = "../sim-contracts" }
sim-state = { path = "../sim-state" }
sim-units = { path = "../sim-units" }

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\subsystem-governance\src\lib.rs") @'
#![forbid(unsafe_code)]

use sim_contracts::EconomyToGovernance;
use sim_state::GovernanceState;
use sim_units::Currency;

pub fn solve_month(
    current: &GovernanceState,
    economy: EconomyToGovernance,
) -> GovernanceState {
    GovernanceState {
        tax_rate: current.tax_rate,
        revenue: Currency(economy.taxable_output.0 * current.tax_rate.0),
        spending: current.spending,
    }
}
'@

# ---------------------------------------------------------------------------
# subsystem-demographics
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\subsystem-demographics\Cargo.toml") @'
[package]
name = "subsystem-demographics"
version.workspace = true
edition.workspace = true

[dependencies]
sim-contracts = { path = "../sim-contracts" }
sim-state = { path = "../sim-state" }

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\subsystem-demographics\src\lib.rs") @'
#![forbid(unsafe_code)]

use sim_contracts::DemographicsOutputs;
use sim_state::DemographicsState;

pub fn advance_month(current: &DemographicsState) -> DemographicsState {
    // Phase 001 intentionally preserves population unchanged.
    // Cohorts, births, deaths, and migration arrive in a later vertical slice.
    current.clone()
}

pub fn publish(state: &DemographicsState) -> DemographicsOutputs {
    DemographicsOutputs {
        population: state.population,
    }
}
'@

# ---------------------------------------------------------------------------
# sim-kernel
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-kernel\Cargo.toml") @'
[package]
name = "sim-kernel"
version.workspace = true
edition.workspace = true

[dependencies]
sim-state = { path = "../sim-state" }
subsystem-demographics = { path = "../subsystem-demographics" }
subsystem-economy = { path = "../subsystem-economy" }
subsystem-energy = { path = "../subsystem-energy" }
subsystem-governance = { path = "../subsystem-governance" }

[lib]
path = "src/lib.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-kernel\src\lib.rs") @'
#![forbid(unsafe_code)]

use sim_state::{StateHash, WorldState};

#[derive(Clone, Copy, Debug)]
pub enum Shock {
    EnergyCapacityLossFraction(f64),
}

#[derive(Clone, Copy, Debug)]
pub struct SolverPolicy {
    pub tolerance: f64,
    pub damping: f64,
    pub max_iterations: u32,
}

impl Default for SolverPolicy {
    fn default() -> Self {
        Self {
            tolerance: 1.0e-8,
            damping: 0.5,
            max_iterations: 64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TickReport {
    pub iterations: u32,
    pub converged: bool,
    pub max_residual: f64,
    pub state_hash: StateHash,
}

#[derive(Clone, Debug, Default)]
pub struct SimulationKernel {
    pub solver: SolverPolicy,
}

impl SimulationKernel {
    pub fn step(
        &self,
        world: &mut WorldState,
        shock: Option<Shock>,
    ) -> Result<TickReport, &'static str> {
        // Phase 1: exogenous primitive input.
        let mut energy = world.energy.clone();
        let mut economy = world.economy.clone();

        if let Some(Shock::EnergyCapacityLossFraction(fraction)) = shock {
            subsystem_energy::apply_capacity_shock(&mut energy, fraction);
        }

        // Phase 2: bounded Jacobi-style coupled solve.
        let mut converged = false;
        let mut max_residual = f64::INFINITY;
        let mut iterations = 0;

        for iteration in 1..=self.solver.max_iterations {
            iterations = iteration;

            // Immutable contract snapshot C[n].
            let economy_to_energy =
                subsystem_economy::publish_to_energy(&economy);
            let energy_to_economy =
                subsystem_energy::publish(&energy);

            // Independent candidates from C[n].
            let energy_candidate =
                subsystem_energy::solve_candidate(&energy, economy_to_energy);
            let economy_candidate =
                subsystem_economy::solve_candidate(&economy, energy_to_economy);

            let gdp_scale = economy.gdp.0.abs().max(1.0);
            let price_scale = energy.price_index.0.abs().max(1.0);

            let gdp_residual =
                (economy_candidate.gdp.0 - economy.gdp.0).abs() / gdp_scale;
            let price_residual =
                (energy_candidate.price_index.0 - energy.price_index.0).abs()
                    / price_scale;

            max_residual = gdp_residual.max(price_residual);

            energy = subsystem_energy::blend(
                &energy,
                &energy_candidate,
                self.solver.damping,
            );
            economy = subsystem_economy::blend(
                &economy,
                &economy_candidate,
                self.solver.damping,
            );

            if max_residual <= self.solver.tolerance {
                converged = true;
                break;
            }
        }

        // Phase 3: slower dependent systems consume the converged/bounded result.
        let economy_to_governance =
            subsystem_economy::publish_to_governance(&economy);
        let governance =
            subsystem_governance::solve_month(&world.governance, economy_to_governance);
        let demographics =
            subsystem_demographics::advance_month(&world.demographics);

        // Phase 4: one logical authoritative commit.
        world.energy = energy;
        world.economy = economy;
        world.governance = governance;
        world.demographics = demographics;
        world.tick = world.tick.next();

        world.validate()?;
        let state_hash = world.canonical_hash();

        Ok(TickReport {
            iterations,
            converged,
            max_residual,
            state_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_input_produces_same_hash() {
        let kernel = SimulationKernel::default();
        let mut a = WorldState::default();
        let mut b = WorldState::default();

        let ra = kernel
            .step(&mut a, Some(Shock::EnergyCapacityLossFraction(0.50)))
            .unwrap();
        let rb = kernel
            .step(&mut b, Some(Shock::EnergyCapacityLossFraction(0.50)))
            .unwrap();

        assert_eq!(ra.state_hash, rb.state_hash);
    }

    #[test]
    fn primitive_energy_shock_propagates_endogenously() {
        let kernel = SimulationKernel::default();
        let mut baseline = WorldState::default();
        let mut shock = baseline.clone();

        kernel.step(&mut baseline, None).unwrap();
        kernel
            .step(&mut shock, Some(Shock::EnergyCapacityLossFraction(0.50)))
            .unwrap();

        assert!(shock.energy.production.0 < baseline.energy.production.0);
        assert!(shock.energy.price_index.0 > baseline.energy.price_index.0);
        assert!(shock.economy.gdp.0 < baseline.economy.gdp.0);
        assert!(shock.governance.revenue.0 < baseline.governance.revenue.0);
    }
}
'@

# ---------------------------------------------------------------------------
# sim-tools
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-tools\Cargo.toml") @'
[package]
name = "sim-tools"
version.workspace = true
edition.workspace = true

[dependencies]
sim-kernel = { path = "../sim-kernel" }
sim-state = { path = "../sim-state" }

[[bin]]
name = "sim-tools"
path = "src/main.rs"
'@

Write-Utf8NoBom (Join-Path $RepoRoot "crates\sim-tools\src\main.rs") @'
#![forbid(unsafe_code)]

use sim_kernel::{Shock, SimulationKernel};
use sim_state::WorldState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let initial = WorldState::default();
    let kernel = SimulationKernel::default();

    let mut baseline = initial.clone();
    let baseline_report = kernel.step(&mut baseline, None)?;

    let mut shock = initial;
    let shock_report =
        kernel.step(&mut shock, Some(Shock::EnergyCapacityLossFraction(0.50)))?;

    println!("New Engine Phase 001 deterministic vertical slice");
    println!();
    println!(
        "baseline: tick={} energy_capacity={:.3} production={:.3} price={:.6} gdp={:.3} revenue={:.3}",
        baseline.tick.0,
        baseline.energy.capacity.0,
        baseline.energy.production.0,
        baseline.energy.price_index.0,
        baseline.economy.gdp.0,
        baseline.governance.revenue.0,
    );
    println!(
        "baseline solver: iterations={} converged={} residual={:.3e}",
        baseline_report.iterations,
        baseline_report.converged,
        baseline_report.max_residual,
    );
    println!("baseline hash: {}", baseline_report.state_hash.to_hex());
    println!();
    println!(
        "shock:    tick={} energy_capacity={:.3} production={:.3} price={:.6} gdp={:.3} revenue={:.3}",
        shock.tick.0,
        shock.energy.capacity.0,
        shock.energy.production.0,
        shock.energy.price_index.0,
        shock.economy.gdp.0,
        shock.governance.revenue.0,
    );
    println!(
        "shock solver: iterations={} converged={} residual={:.3e}",
        shock_report.iterations,
        shock_report.converged,
        shock_report.max_residual,
    );
    println!("shock hash: {}", shock_report.state_hash.to_hex());

    Ok(())
}
'@

# ---------------------------------------------------------------------------
# Implementation note
# ---------------------------------------------------------------------------

Write-Utf8NoBom (Join-Path $RepoRoot "docs\implementation\PHASE-001-CARGO-WORKSPACE.md") @'
# Phase 001 — Cargo Workspace and Deterministic Vertical Slice

## Purpose

Phase 001 turns the architecture documents into compiling Rust boundaries.

It deliberately implements only a tiny causal demonstration:

```text
primitive energy-capacity shock
 -> energy shortage / price response
 -> economy response
 -> governance revenue response
 -> authoritative commit
 -> BLAKE3 state hash
```

The equations in this phase are engineering placeholders.

They are **not** copied from International Futures (IFs), are **not** calibrated,
and are not intended to become the production equations merely because they
exist first.

IFs remains an architecture/behavior reference. CMO remains a military
operational-relationship reference at the higher abstraction defined in
ADR-008.

## Workspace

- `sim-types`
- `sim-storage`
- `sim-units`
- `sim-equations`
- `sim-contracts`
- `sim-state`
- `sim-kernel`
- `sim-random`
- `sim-tools`
- `subsystem-demographics`
- `subsystem-energy`
- `subsystem-economy`
- `subsystem-governance`

## Architecture demonstrated

- Rust 2024 Cargo workspace
- pinned stable toolchain channel
- typed subsystem boundaries
- single authoritative world commit per monthly tick
- primitive cause before downstream outcome
- Jacobi-style Economy/Energy contract snapshot
- deterministic damping and iteration cap
- subsystem ownership boundaries
- BLAKE3 canonical state hash
- baseline-vs-shock isolation
- headless CLI execution
- dense/sparse storage primitives without graphics dependency

## What is intentionally absent

- calibrated IFs-like equations
- agriculture
- infrastructure networks
- trade
- full demographic cohorts
- delayed pipelines
- persistence/replay files
- production PRNG implementation
- military formations/substeps
- UI

Those are added incrementally behind the architecture already committed.
'@

# Keep the bootstrap script in the repository for reproducibility.
New-Item -ItemType Directory -Force -Path (Join-Path $RepoRoot "tools") | Out-Null
$bootstrapDest = Join-Path $RepoRoot "tools\bootstrap-phase-001.ps1"
if ($PSCommandPath) {
    $sourcePath = [System.IO.Path]::GetFullPath($PSCommandPath)
    $destPath = [System.IO.Path]::GetFullPath($bootstrapDest)
    if ($sourcePath -ne $destPath) {
        Copy-Item -Force $sourcePath $destPath
    }
}

Set-Location $RepoRoot

Write-Host ""
Write-Host "Running Rust validation..."
cargo fmt --all
cargo check --workspace
cargo test --workspace
cargo run -p sim-tools

Write-Host ""
Write-Host "Phase 001 scaffold completed successfully."
Write-Host "Review with: git status"
