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
