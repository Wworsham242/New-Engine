#![forbid(unsafe_code)]

pub mod metadata;

use sim_contracts::{EconomyToEnergy, EnergyToEconomy};
use sim_state::EnergyState;
use sim_types::{CountryId, EnergyTypeId};
use sim_units::{EnergyQuantity, PriceIndex, Rate};

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
        production: total_production(state, country),
        demand: *state.demand.get(country.index()).unwrap(),
        price_index: *state.price_index.get(country.index()).unwrap(),
        shortage_fraction: *state.shortage_fraction.get(country.index()).unwrap(),
    }
}

pub fn total_capacity(state: &EnergyState, country: CountryId) -> EnergyQuantity {
    EnergyQuantity(
        state
            .capacity_by_type
            .row(country.index())
            .expect("country must exist")
            .iter()
            .map(|x| x.0)
            .sum(),
    )
}

pub fn total_production(state: &EnergyState, country: CountryId) -> EnergyQuantity {
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

/// Phase-001/003 scaffold equation only.
///
/// This remains deliberately simple and uncalibrated. It proves country-indexed
/// causal and solver architecture; it is not an IFs equation.
pub fn solve_country_candidate(
    current: &EnergyState,
    country: CountryId,
    economy: EconomyToEnergy,
) -> (Vec<EnergyQuantity>, EnergyQuantity, PriceIndex, Rate) {
    let capacity_row = current
        .capacity_by_type
        .row(country.index())
        .expect("country must exist");
    let total_capacity: f64 = capacity_row.iter().map(|x| x.0).sum();
    let demand = (economy.gdp.0 / 10.0).max(1.0);
    let production_total = total_capacity.min(demand).max(0.0);
    let shortage = ((demand - production_total) / demand).clamp(0.0, 1.0);
    let price = 1.0 + 2.0 * shortage;

    let production = if total_capacity > 0.0 {
        capacity_row
            .iter()
            .map(|capacity| EnergyQuantity(production_total * (capacity.0 / total_capacity)))
            .collect()
    } else {
        vec![EnergyQuantity(0.0); capacity_row.len()]
    };

    (
        production,
        EnergyQuantity(demand),
        PriceIndex(price),
        Rate(shortage),
    )
}

pub fn blend_country(
    state: &mut EnergyState,
    country: CountryId,
    candidate_production: &[EnergyQuantity],
    candidate_demand: EnergyQuantity,
    candidate_price: PriceIndex,
    candidate_shortage: Rate,
    damping: f64,
) {
    let a = damping.clamp(0.0, 1.0);
    let lerp = |old: f64, new: f64| old + a * (new - old);

    let production_row = state
        .production_by_type
        .row_mut(country.index())
        .expect("country must exist");

    assert_eq!(production_row.len(), candidate_production.len());
    for (current, candidate) in production_row.iter_mut().zip(candidate_production) {
        current.0 = lerp(current.0, candidate.0);
    }

    let demand = state.demand.get_mut(country.index()).unwrap();
    demand.0 = lerp(demand.0, candidate_demand.0);

    let price = state.price_index.get_mut(country.index()).unwrap();
    price.0 = lerp(price.0, candidate_price.0);

    let shortage = state.shortage_fraction.get_mut(country.index()).unwrap();
    shortage.0 = lerp(shortage.0, candidate_shortage.0);
}
