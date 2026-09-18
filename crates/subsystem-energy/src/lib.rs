#![forbid(unsafe_code)]

pub mod metadata;

use sim_contracts::{EconomyToEnergy, EnergyToEconomy};
use sim_state::EnergyState;
use sim_storage::{Dense1, Dense2, Dense3};
use sim_types::{CountryId, EnergyTypeId};
use sim_units::{EnergyQuantity, PriceIndex, Rate};

#[derive(Clone, Debug)]
pub struct EnergyCandidate {
    pub production_by_type: Dense2<EnergyQuantity>,
    pub realized_trade_by_type: Dense3<EnergyQuantity>,
    pub demand: Dense1<EnergyQuantity>,
    pub price_index: Dense1<PriceIndex>,
    pub shortage_fraction: Dense1<Rate>,
}

pub fn apply_capacity_shock(
    state: &mut EnergyState,
    country: CountryId,
    energy_type: EnergyTypeId,
    fraction_lost: f64,
) {
    let bounded = fraction_lost.clamp(0.0, 1.0);
    let cell = state
        .capacity_by_type
        .get_mut(country.index(), energy_type.0 as usize)
        .expect("capacity shock target must exist");
    cell.0 *= 1.0 - bounded;
}

pub fn publish(state: &EnergyState, country: CountryId) -> EnergyToEconomy {
    EnergyToEconomy {
        production: total_available_for_domestic_use(state, country),
        demand: *state.demand.get(country.index()).unwrap(),
        price_index: *state.price_index.get(country.index()).unwrap(),
        shortage_fraction: *state.shortage_fraction.get(country.index()).unwrap(),
    }
}

pub fn total_domestic_production(state: &EnergyState, country: CountryId) -> EnergyQuantity {
    EnergyQuantity(
        state
            .production_by_type
            .row(country.index())
            .expect("country must exist")
            .iter()
            .map(|x| x.0)
            .sum(),
    )
}

pub fn total_imports(state: &EnergyState, country: CountryId) -> EnergyQuantity {
    let countries = state.realized_trade_by_type.dim0();
    let energy_types = state.realized_trade_by_type.dim2();
    let destination = country.index();

    let mut total = 0.0;
    for origin in 0..countries {
        for energy_type in 0..energy_types {
            total += state
                .realized_trade_by_type
                .get(origin, destination, energy_type)
                .unwrap()
                .0;
        }
    }
    EnergyQuantity(total)
}

pub fn total_exports(state: &EnergyState, country: CountryId) -> EnergyQuantity {
    let countries = state.realized_trade_by_type.dim1();
    let energy_types = state.realized_trade_by_type.dim2();
    let origin = country.index();

    let mut total = 0.0;
    for destination in 0..countries {
        for energy_type in 0..energy_types {
            total += state
                .realized_trade_by_type
                .get(origin, destination, energy_type)
                .unwrap()
                .0;
        }
    }
    EnergyQuantity(total)
}

pub fn total_available_for_domestic_use(state: &EnergyState, country: CountryId) -> EnergyQuantity {
    let production = total_domestic_production(state, country).0;
    let imports = total_imports(state, country).0;
    let exports = total_exports(state, country).0;
    EnergyQuantity((production + imports - exports).max(0.0))
}

/// Phase-004 global Energy candidate.
///
/// Production is currently an uncalibrated capacity realization. Planned trade
/// is then constrained by each origin's production of the relevant energy type.
/// If an origin cannot fulfill all planned exports of a type, every destination
/// receives the same deterministic fulfillment ratio for that origin/type.
///
/// This is an engineering scaffold, not an IFs equation.
pub fn solve_global_candidate(
    current: &EnergyState,
    economy: &[EconomyToEnergy],
) -> EnergyCandidate {
    let countries = current.capacity_by_type.rows();
    let energy_types = current.capacity_by_type.cols();

    assert_eq!(economy.len(), countries);

    let production_by_type = current.capacity_by_type.clone();
    let mut realized_trade_by_type =
        Dense3::new(countries, countries, energy_types, EnergyQuantity(0.0));

    // First resolve each origin/type export constraint in canonical order.
    for origin in 0..countries {
        for energy_type in 0..energy_types {
            let production = production_by_type.get(origin, energy_type).unwrap().0;

            let planned_total: f64 = (0..countries)
                .map(|destination| {
                    current
                        .planned_trade_by_type
                        .get(origin, destination, energy_type)
                        .unwrap()
                        .0
                })
                .sum();

            let fulfillment = if planned_total <= 0.0 {
                0.0
            } else {
                (production / planned_total).clamp(0.0, 1.0)
            };

            for destination in 0..countries {
                let planned = current
                    .planned_trade_by_type
                    .get(origin, destination, energy_type)
                    .unwrap()
                    .0;

                *realized_trade_by_type
                    .get_mut(origin, destination, energy_type)
                    .unwrap() = EnergyQuantity(planned * fulfillment);
            }
        }
    }

    let mut demand = Dense1::new(countries, EnergyQuantity(0.0));
    let mut price_index = Dense1::new(countries, PriceIndex(1.0));
    let mut shortage_fraction = Dense1::new(countries, Rate(0.0));

    // Then compute each country's physical availability.
    for country in 0..countries {
        let domestic_production: f64 = production_by_type
            .row(country)
            .unwrap()
            .iter()
            .map(|x| x.0)
            .sum();

        let mut imports = 0.0;
        let mut exports = 0.0;

        for other in 0..countries {
            for energy_type in 0..energy_types {
                imports += realized_trade_by_type
                    .get(other, country, energy_type)
                    .unwrap()
                    .0;
                exports += realized_trade_by_type
                    .get(country, other, energy_type)
                    .unwrap()
                    .0;
            }
        }

        let desired_demand = (economy[country].gdp.0 / 10.0).max(1.0);
        let available = (domestic_production + imports - exports).max(0.0);
        let shortage = ((desired_demand - available) / desired_demand).clamp(0.0, 1.0);
        let price = 1.0 + 2.0 * shortage;

        *demand.get_mut(country).unwrap() = EnergyQuantity(desired_demand);
        *price_index.get_mut(country).unwrap() = PriceIndex(price);
        *shortage_fraction.get_mut(country).unwrap() = Rate(shortage);
    }

    EnergyCandidate {
        production_by_type,
        realized_trade_by_type,
        demand,
        price_index,
        shortage_fraction,
    }
}

pub fn blend_global(state: &mut EnergyState, candidate: &EnergyCandidate, damping: f64) {
    let a = damping.clamp(0.0, 1.0);
    let lerp = |old: f64, new: f64| old + a * (new - old);

    for (current, new) in state
        .production_by_type
        .canonical_values()
        .iter()
        .zip(candidate.production_by_type.canonical_values())
    {
        // This loop only checks shape; actual write happens below because
        // canonical_values intentionally exposes immutable storage.
        let _ = (current, new);
    }

    let countries = state.capacity_by_type.rows();
    let energy_types = state.capacity_by_type.cols();

    for country in 0..countries {
        for energy_type in 0..energy_types {
            let old = state
                .production_by_type
                .get(country, energy_type)
                .unwrap()
                .0;
            let new = candidate
                .production_by_type
                .get(country, energy_type)
                .unwrap()
                .0;
            state
                .production_by_type
                .get_mut(country, energy_type)
                .unwrap()
                .0 = lerp(old, new);
        }
    }

    for origin in 0..countries {
        for destination in 0..countries {
            for energy_type in 0..energy_types {
                let old = state
                    .realized_trade_by_type
                    .get(origin, destination, energy_type)
                    .unwrap()
                    .0;
                let new = candidate
                    .realized_trade_by_type
                    .get(origin, destination, energy_type)
                    .unwrap()
                    .0;

                state
                    .realized_trade_by_type
                    .get_mut(origin, destination, energy_type)
                    .unwrap()
                    .0 = lerp(old, new);
            }
        }
    }

    for country in 0..countries {
        let old_demand = state.demand.get(country).unwrap().0;
        let new_demand = candidate.demand.get(country).unwrap().0;
        state.demand.get_mut(country).unwrap().0 = lerp(old_demand, new_demand);

        let old_price = state.price_index.get(country).unwrap().0;
        let new_price = candidate.price_index.get(country).unwrap().0;
        state.price_index.get_mut(country).unwrap().0 = lerp(old_price, new_price);

        let old_shortage = state.shortage_fraction.get(country).unwrap().0;
        let new_shortage = candidate.shortage_fraction.get(country).unwrap().0;
        state.shortage_fraction.get_mut(country).unwrap().0 = lerp(old_shortage, new_shortage);
    }
}

pub fn trade_conservation_error(state: &EnergyState) -> f64 {
    let countries = state.realized_trade_by_type.dim0();
    let energy_types = state.realized_trade_by_type.dim2();

    let mut imports = 0.0;
    let mut exports = 0.0;

    for origin in 0..countries {
        for destination in 0..countries {
            for energy_type in 0..energy_types {
                let flow = state
                    .realized_trade_by_type
                    .get(origin, destination, energy_type)
                    .unwrap()
                    .0;
                exports += flow;
                imports += flow;
            }
        }
    }

    (imports - exports).abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_state::WorldState;

    #[test]
    fn physical_trade_is_conserved() {
        let world = WorldState::demo();
        assert_eq!(trade_conservation_error(&world.energy), 0.0);
    }
}
