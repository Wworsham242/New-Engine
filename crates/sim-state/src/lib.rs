#![forbid(unsafe_code)]

use sim_registry::WorldRegistry;
use sim_storage::{Dense1, Dense2, Dense3};
use sim_types::{CountryId, EnergyTypeId, Tick};
use sim_units::{Currency, EnergyQuantity, Population, PriceIndex, Rate, RealGdp};

#[derive(Clone, Debug)]
pub struct EnergyState {
    pub capacity_by_type: Dense2<EnergyQuantity>,
    pub production_by_type: Dense2<EnergyQuantity>,

    /// Desired physical flows [origin, destination, energy_type].
    ///
    /// This is an authoritative commitment/plan in Phase 004.
    pub planned_trade_by_type: Dense3<EnergyQuantity>,

    /// Actually deliverable physical flows after origin production constraints.
    pub realized_trade_by_type: Dense3<EnergyQuantity>,

    pub demand: Dense1<EnergyQuantity>,
    pub price_index: Dense1<PriceIndex>,
    pub shortage_fraction: Dense1<Rate>,
}

#[derive(Clone, Debug)]
pub struct EconomyState {
    pub gdp: Dense1<RealGdp>,
    pub potential_gdp: Dense1<RealGdp>,
    pub investment: Dense1<Currency>,
}

#[derive(Clone, Debug)]
pub struct GovernanceState {
    pub tax_rate: Dense1<Rate>,
    pub revenue: Dense1<Currency>,
    pub spending: Dense1<Currency>,
}

#[derive(Clone, Debug)]
pub struct DemographicsState {
    pub population: Dense1<Population>,
}

#[derive(Clone, Debug)]
pub struct WorldState {
    pub tick: Tick,
    pub registry: WorldRegistry,
    pub energy: EnergyState,
    pub economy: EconomyState,
    pub governance: GovernanceState,
    pub demographics: DemographicsState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateHash(pub [u8; 32]);

impl StateHash {
    pub fn to_hex(self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }
}

impl WorldState {
    pub fn demo() -> Self {
        let registry = WorldRegistry::builder()
            .country("USA", "United States")
            .country("CAN", "Canada")
            .country("SAU", "Saudi Arabia")
            .region("WORLD", "World")
            .sector("aggregate", "Aggregate Economy")
            .energy_type("oil", "Oil")
            .energy_type("gas", "Natural Gas")
            .energy_type("electricity", "Electricity")
            .build()
            .expect("demo registry must be valid");

        let countries = registry.countries().len();
        let energy_types = registry.energy_types().len();

        let mut capacity_by_type = Dense2::new(countries, energy_types, EnergyQuantity(0.0));
        let mut production_by_type = Dense2::new(countries, energy_types, EnergyQuantity(0.0));

        // Engineering fixture only; not real-world calibrated data.
        let capacities = [
            [45.0, 30.0, 25.0], // USA total 100
            [20.0, 15.0, 15.0], // CAN total 50
            [50.0, 5.0, 5.0],   // SAU total 60
        ];

        for (country, row) in capacities.iter().enumerate() {
            for (energy_type, value) in row.iter().enumerate() {
                *capacity_by_type.get_mut(country, energy_type).unwrap() = EnergyQuantity(*value);
                *production_by_type.get_mut(country, energy_type).unwrap() = EnergyQuantity(*value);
            }
        }

        let mut planned_trade_by_type =
            Dense3::new(countries, countries, energy_types, EnergyQuantity(0.0));

        // Balanced physical oil trade fixture:
        // SAU -> USA = 30
        // USA -> CAN = 30
        //
        // With domestic production totals and demand:
        // USA: 100 + 30 - 30 = 100
        // CAN:  50 + 30      = 80
        // SAU:  60      - 30 = 30
        //
        // A USA oil-capacity shock can therefore reduce USA exports to Canada,
        // transmitting the physical shortage internationally.
        let usa = registry
            .countries()
            .iter()
            .find(|x| x.key == "USA")
            .unwrap()
            .id;
        let can = registry
            .countries()
            .iter()
            .find(|x| x.key == "CAN")
            .unwrap()
            .id;
        let sau = registry
            .countries()
            .iter()
            .find(|x| x.key == "SAU")
            .unwrap()
            .id;
        let oil = registry
            .energy_types()
            .iter()
            .find(|x| x.key == "oil")
            .unwrap()
            .id;

        *planned_trade_by_type
            .get_mut(sau.index(), usa.index(), oil.0 as usize)
            .unwrap() = EnergyQuantity(30.0);
        *planned_trade_by_type
            .get_mut(usa.index(), can.index(), oil.0 as usize)
            .unwrap() = EnergyQuantity(30.0);

        let realized_trade_by_type = planned_trade_by_type.clone();

        Self {
            tick: Tick(0),
            registry,
            energy: EnergyState {
                capacity_by_type,
                production_by_type,
                planned_trade_by_type,
                realized_trade_by_type,
                demand: Dense1::from_vec(vec![
                    EnergyQuantity(100.0),
                    EnergyQuantity(80.0),
                    EnergyQuantity(30.0),
                ]),
                price_index: Dense1::from_vec(vec![
                    PriceIndex(1.0),
                    PriceIndex(1.0),
                    PriceIndex(1.0),
                ]),
                shortage_fraction: Dense1::from_vec(vec![Rate(0.0), Rate(0.0), Rate(0.0)]),
            },
            economy: EconomyState {
                gdp: Dense1::from_vec(vec![RealGdp(1_000.0), RealGdp(800.0), RealGdp(300.0)]),
                potential_gdp: Dense1::from_vec(vec![
                    RealGdp(1_000.0),
                    RealGdp(800.0),
                    RealGdp(300.0),
                ]),
                investment: Dense1::from_vec(vec![
                    Currency(200.0),
                    Currency(160.0),
                    Currency(60.0),
                ]),
            },
            governance: GovernanceState {
                tax_rate: Dense1::from_vec(vec![Rate(0.20), Rate(0.20), Rate(0.20)]),
                revenue: Dense1::from_vec(vec![Currency(200.0), Currency(160.0), Currency(60.0)]),
                spending: Dense1::from_vec(vec![Currency(220.0), Currency(175.0), Currency(70.0)]),
            },
            demographics: DemographicsState {
                population: Dense1::from_vec(vec![
                    Population(340_000_000.0),
                    Population(41_000_000.0),
                    Population(35_000_000.0),
                ]),
            },
        }
    }

    pub fn country_count(&self) -> usize {
        self.registry.countries().len()
    }

    pub fn energy_type_count(&self) -> usize {
        self.registry.energy_types().len()
    }

    pub fn country_id_by_key(&self, key: &str) -> Option<CountryId> {
        self.registry
            .countries()
            .iter()
            .find(|record| record.key == key)
            .map(|record| record.id)
    }

    pub fn energy_type_id_by_key(&self, key: &str) -> Option<EnergyTypeId> {
        self.registry
            .energy_types()
            .iter()
            .find(|record| record.key == key)
            .map(|record| record.id)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.registry
            .validate()
            .map_err(|_| "world registry invalid")?;

        let countries = self.country_count();
        let energy_types = self.energy_type_count();

        if self.economy.gdp.len() != countries
            || self.economy.potential_gdp.len() != countries
            || self.economy.investment.len() != countries
            || self.governance.tax_rate.len() != countries
            || self.governance.revenue.len() != countries
            || self.governance.spending.len() != countries
            || self.demographics.population.len() != countries
            || self.energy.demand.len() != countries
            || self.energy.price_index.len() != countries
            || self.energy.shortage_fraction.len() != countries
            || self.energy.capacity_by_type.rows() != countries
            || self.energy.production_by_type.rows() != countries
            || self.energy.capacity_by_type.cols() != energy_types
            || self.energy.production_by_type.cols() != energy_types
            || self.energy.planned_trade_by_type.dim0() != countries
            || self.energy.planned_trade_by_type.dim1() != countries
            || self.energy.planned_trade_by_type.dim2() != energy_types
            || self.energy.realized_trade_by_type.dim0() != countries
            || self.energy.realized_trade_by_type.dim1() != countries
            || self.energy.realized_trade_by_type.dim2() != energy_types
        {
            return Err("dense state dimensions do not match world registry");
        }

        for value in self
            .energy
            .capacity_by_type
            .canonical_values()
            .iter()
            .map(|x| x.0)
            .chain(
                self.energy
                    .production_by_type
                    .canonical_values()
                    .iter()
                    .map(|x| x.0),
            )
            .chain(
                self.energy
                    .planned_trade_by_type
                    .canonical_values()
                    .iter()
                    .map(|x| x.0),
            )
            .chain(
                self.energy
                    .realized_trade_by_type
                    .canonical_values()
                    .iter()
                    .map(|x| x.0),
            )
            .chain(self.energy.demand.as_slice().iter().map(|x| x.0))
            .chain(self.energy.price_index.as_slice().iter().map(|x| x.0))
            .chain(self.energy.shortage_fraction.as_slice().iter().map(|x| x.0))
            .chain(self.economy.gdp.as_slice().iter().map(|x| x.0))
            .chain(self.economy.potential_gdp.as_slice().iter().map(|x| x.0))
            .chain(self.economy.investment.as_slice().iter().map(|x| x.0))
            .chain(self.governance.tax_rate.as_slice().iter().map(|x| x.0))
            .chain(self.governance.revenue.as_slice().iter().map(|x| x.0))
            .chain(self.governance.spending.as_slice().iter().map(|x| x.0))
            .chain(self.demographics.population.as_slice().iter().map(|x| x.0))
        {
            if !value.is_finite() {
                return Err("authoritative state contains a non-finite value");
            }
            if value < 0.0 {
                return Err("authoritative state contains a negative quantity");
            }
        }

        // No self-trade in the Phase-004 physical flow matrix.
        for country in 0..countries {
            for energy_type in 0..energy_types {
                if self
                    .energy
                    .planned_trade_by_type
                    .get(country, country, energy_type)
                    .unwrap()
                    .0
                    != 0.0
                    || self
                        .energy
                        .realized_trade_by_type
                        .get(country, country, energy_type)
                        .unwrap()
                        .0
                        != 0.0
                {
                    return Err("self-trade is not permitted");
                }
            }
        }

        Ok(())
    }

    pub fn canonical_hash(&self) -> StateHash {
        let mut hasher = blake3::Hasher::new();

        hasher.update(b"new-engine.world-state.phase004.v1");
        hasher.update(&self.tick.0.to_le_bytes());

        for country in self.registry.countries() {
            hasher.update(&country.id.0.to_le_bytes());
            hash_string(&mut hasher, &country.key);
            hash_string(&mut hasher, &country.name);
        }
        for energy_type in self.registry.energy_types() {
            hasher.update(&energy_type.id.0.to_le_bytes());
            hash_string(&mut hasher, &energy_type.key);
            hash_string(&mut hasher, &energy_type.name);
        }

        hash_f64s(
            &mut hasher,
            self.energy
                .capacity_by_type
                .canonical_values()
                .iter()
                .map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.energy
                .production_by_type
                .canonical_values()
                .iter()
                .map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.energy
                .planned_trade_by_type
                .canonical_values()
                .iter()
                .map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.energy
                .realized_trade_by_type
                .canonical_values()
                .iter()
                .map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.energy.demand.as_slice().iter().map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.energy.price_index.as_slice().iter().map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.energy.shortage_fraction.as_slice().iter().map(|x| x.0),
        );
        hash_f64s(&mut hasher, self.economy.gdp.as_slice().iter().map(|x| x.0));
        hash_f64s(
            &mut hasher,
            self.economy.potential_gdp.as_slice().iter().map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.economy.investment.as_slice().iter().map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.governance.tax_rate.as_slice().iter().map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.governance.revenue.as_slice().iter().map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.governance.spending.as_slice().iter().map(|x| x.0),
        );
        hash_f64s(
            &mut hasher,
            self.demographics.population.as_slice().iter().map(|x| x.0),
        );

        StateHash(*hasher.finalize().as_bytes())
    }
}

fn hash_string(hasher: &mut blake3::Hasher, value: &str) {
    let bytes = value.as_bytes();
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(bytes);
}

fn hash_f64s(hasher: &mut blake3::Hasher, values: impl IntoIterator<Item = f64>) {
    for value in values {
        hasher.update(&value.to_bits().to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_trade_world_has_equal_hash() {
        let a = WorldState::demo();
        let b = a.clone();
        assert_eq!(a.canonical_hash(), b.canonical_hash());
    }

    #[test]
    fn trade_change_changes_hash() {
        let a = WorldState::demo();
        let mut b = a.clone();

        let usa = b.country_id_by_key("USA").unwrap();
        let can = b.country_id_by_key("CAN").unwrap();
        let oil = b.energy_type_id_by_key("oil").unwrap();

        b.energy
            .planned_trade_by_type
            .get_mut(usa.index(), can.index(), oil.0 as usize)
            .unwrap()
            .0 -= 1.0;

        assert_ne!(a.canonical_hash(), b.canonical_hash());
    }

    #[test]
    fn registry_dimensions_validate() {
        let world = WorldState::demo();
        world.validate().unwrap();
        assert_eq!(world.country_count(), 3);
        assert_eq!(world.energy_type_count(), 3);
    }
}
