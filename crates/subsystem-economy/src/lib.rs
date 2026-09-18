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
pub fn solve_candidate(current: &EconomyState, energy: EnergyToEconomy) -> EconomyState {
    let availability_factor = (1.0 - 0.25 * energy.shortage_fraction.0).clamp(0.0, 1.0);

    EconomyState {
        gdp: RealGdp(current.potential_gdp.0 * availability_factor),
        potential_gdp: current.potential_gdp,
        investment: current.investment,
    }
}

pub fn blend(previous: &EconomyState, candidate: &EconomyState, damping: f64) -> EconomyState {
    let a = damping.clamp(0.0, 1.0);
    EconomyState {
        gdp: RealGdp(previous.gdp.0 + a * (candidate.gdp.0 - previous.gdp.0)),
        potential_gdp: candidate.potential_gdp,
        investment: candidate.investment,
    }
}
