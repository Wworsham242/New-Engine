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
pub fn solve_candidate(current: &EnergyState, economy: EconomyToEnergy) -> EnergyState {
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

pub fn blend(previous: &EnergyState, candidate: &EnergyState, damping: f64) -> EnergyState {
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
