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
    pub next_in_transit_trade_by_type: Dense3<EnergyQuantity>,
    pub next_inventory_by_type: Dense2<EnergyQuantity>,
    pub inventory_draw_by_type: Dense2<EnergyQuantity>,
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
        production: total_effective_supply(state, country),
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

pub fn total_arrivals(state: &EnergyState, country: CountryId) -> EnergyQuantity {
    let countries = state.in_transit_trade_by_type.dim0();
    let energy_types = state.in_transit_trade_by_type.dim2();
    let destination = country.index();

    let mut total = 0.0;
    for origin in 0..countries {
        for energy_type in 0..energy_types {
            total += state
                .in_transit_trade_by_type
                .get(origin, destination, energy_type)
                .unwrap()
                .0;
        }
    }
    EnergyQuantity(total)
}

pub fn total_exports_launched(state: &EnergyState, country: CountryId) -> EnergyQuantity {
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

pub fn total_inventory_draw(state: &EnergyState, country: CountryId) -> EnergyQuantity {
    EnergyQuantity(
        state
            .inventory_draw_by_type
            .row(country.index())
            .unwrap()
            .iter()
            .map(|x| x.0)
            .sum(),
    )
}

pub fn total_effective_supply(state: &EnergyState, country: CountryId) -> EnergyQuantity {
    EnergyQuantity(
        (total_domestic_production(state, country).0 + total_arrivals(state, country).0
            - total_exports_launched(state, country).0
            + total_inventory_draw(state, country).0)
            .max(0.0),
    )
}

/// Build one global candidate from the committed temporal base.
///
/// Important ADR-005 invariant:
/// solver iteration does not advance transit or deplete inventory multiple
/// times. `temporal_base` remains fixed throughout the outer solve. Only the
/// current economic contract changes between iterations.
pub fn solve_global_candidate(
    temporal_base: &EnergyState,
    economy: &[EconomyToEnergy],
) -> EnergyCandidate {
    let countries = temporal_base.capacity_by_type.rows();
    let energy_types = temporal_base.capacity_by_type.cols();

    assert_eq!(economy.len(), countries);

    let production_by_type = temporal_base.capacity_by_type.clone();
    let mut realized_trade_by_type =
        Dense3::new(countries, countries, energy_types, EnergyQuantity(0.0));

    // Resolve shipments launched this tick from physical origin/type production.
    for origin in 0..countries {
        for energy_type in 0..energy_types {
            let production = production_by_type.get(origin, energy_type).unwrap().0;

            let planned_total: f64 = (0..countries)
                .map(|destination| {
                    temporal_base
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
                let planned = temporal_base
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

    // Shipments launched now become next tick's in-transit arrivals.
    let next_in_transit_trade_by_type = realized_trade_by_type.clone();

    let mut demand = Dense1::new(countries, EnergyQuantity(0.0));
    let mut price_index = Dense1::new(countries, PriceIndex(1.0));
    let mut shortage_fraction = Dense1::new(countries, Rate(0.0));
    let mut next_inventory_by_type = temporal_base.inventory_by_type.clone();
    let mut inventory_draw_by_type = Dense2::new(countries, energy_types, EnergyQuantity(0.0));

    for (country, economy_contract) in economy.iter().enumerate() {
        let domestic_production: f64 = production_by_type
            .row(country)
            .unwrap()
            .iter()
            .map(|x| x.0)
            .sum();

        let mut arrivals = 0.0;
        let mut launched_exports = 0.0;

        for other in 0..countries {
            for energy_type in 0..energy_types {
                arrivals += temporal_base
                    .in_transit_trade_by_type
                    .get(other, country, energy_type)
                    .unwrap()
                    .0;

                launched_exports += realized_trade_by_type
                    .get(country, other, energy_type)
                    .unwrap()
                    .0;
            }
        }

        let desired_demand = (economy_contract.gdp.0 / 10.0).max(1.0);
        let pre_buffer_supply = (domestic_production + arrivals - launched_exports).max(0.0);
        let gap = (desired_demand - pre_buffer_supply).max(0.0);

        // Draw inventory deterministically in ascending EnergyTypeId until the
        // aggregate physical gap is covered or stocks are exhausted.
        let mut remaining_gap = gap;
        for energy_type in 0..energy_types {
            if remaining_gap <= 0.0 {
                break;
            }

            let stock = temporal_base
                .inventory_by_type
                .get(country, energy_type)
                .unwrap()
                .0;

            let draw = stock.min(remaining_gap).max(0.0);
            inventory_draw_by_type
                .get_mut(country, energy_type)
                .unwrap()
                .0 = draw;
            next_inventory_by_type
                .get_mut(country, energy_type)
                .unwrap()
                .0 = stock - draw;
            remaining_gap -= draw;
        }

        let total_draw: f64 = inventory_draw_by_type
            .row(country)
            .unwrap()
            .iter()
            .map(|x| x.0)
            .sum();

        let effective_supply = pre_buffer_supply + total_draw;
        let shortage = ((desired_demand - effective_supply) / desired_demand).clamp(0.0, 1.0);
        let price = 1.0 + 2.0 * shortage;

        *demand.get_mut(country).unwrap() = EnergyQuantity(desired_demand);
        *price_index.get_mut(country).unwrap() = PriceIndex(price);
        *shortage_fraction.get_mut(country).unwrap() = Rate(shortage);
    }

    EnergyCandidate {
        production_by_type,
        realized_trade_by_type,
        next_in_transit_trade_by_type,
        next_inventory_by_type,
        inventory_draw_by_type,
        demand,
        price_index,
        shortage_fraction,
    }
}

/// Apply a numerical candidate without advancing physical time more than once.
///
/// Physical stock/flow candidates are replaced exactly from the temporal-base
/// calculation. Only iterative response variables are damped.
pub fn apply_candidate(state: &mut EnergyState, candidate: &EnergyCandidate, damping: f64) {
    state.production_by_type = candidate.production_by_type.clone();
    state.realized_trade_by_type = candidate.realized_trade_by_type.clone();
    state.in_transit_trade_by_type = candidate.next_in_transit_trade_by_type.clone();
    state.inventory_by_type = candidate.next_inventory_by_type.clone();
    state.inventory_draw_by_type = candidate.inventory_draw_by_type.clone();

    let a = damping.clamp(0.0, 1.0);
    let lerp = |old: f64, new: f64| old + a * (new - old);

    for country in 0..state.demand.len() {
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

    #[test]
    fn candidate_never_draws_more_inventory_than_exists() {
        let world = WorldState::demo();
        let economy: Vec<_> = world
            .registry
            .countries()
            .iter()
            .map(|country| sim_contracts::EconomyToEnergy {
                gdp: *world.economy.gdp.get(country.id.index()).unwrap(),
                investment: *world.economy.investment.get(country.id.index()).unwrap(),
            })
            .collect();

        let candidate = solve_global_candidate(&world.energy, &economy);

        for country in 0..world.country_count() {
            for energy_type in 0..world.energy_type_count() {
                let before = world
                    .energy
                    .inventory_by_type
                    .get(country, energy_type)
                    .unwrap()
                    .0;
                let draw = candidate
                    .inventory_draw_by_type
                    .get(country, energy_type)
                    .unwrap()
                    .0;
                let after = candidate
                    .next_inventory_by_type
                    .get(country, energy_type)
                    .unwrap()
                    .0;

                assert!(draw <= before + 1.0e-12);
                assert!(after >= -1.0e-12);
                assert!((before - draw - after).abs() <= 1.0e-12);
            }
        }
    }
}
