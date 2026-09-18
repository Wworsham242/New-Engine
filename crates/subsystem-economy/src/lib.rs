#![forbid(unsafe_code)]

pub mod metadata;

use sim_contracts::{EconomyToEnergy, EconomyToGovernance, EnergyToEconomy};
use sim_state::EconomyState;
use sim_types::CountryId;
use sim_units::RealGdp;

pub fn publish_to_energy(state: &EconomyState, country: CountryId) -> EconomyToEnergy {
    EconomyToEnergy {
        gdp: *state.gdp.get(country.index()).unwrap(),
        investment: *state.investment.get(country.index()).unwrap(),
    }
}

pub fn publish_to_governance(state: &EconomyState, country: CountryId) -> EconomyToGovernance {
    EconomyToGovernance {
        taxable_output: *state.gdp.get(country.index()).unwrap(),
    }
}

/// Phase-001/003 scaffold equation only.
///
/// The coefficient remains an engineering placeholder used to prove a
/// country-indexed Energy <-> Economy solve. It is not calibrated or copied
/// from IFs.
pub fn solve_country_candidate(
    current: &EconomyState,
    country: CountryId,
    energy: EnergyToEconomy,
) -> RealGdp {
    let availability_factor = (1.0 - 0.25 * energy.shortage_fraction.0).clamp(0.0, 1.0);
    let potential = current.potential_gdp.get(country.index()).unwrap().0;
    RealGdp(potential * availability_factor)
}

pub fn blend_country(
    state: &mut EconomyState,
    country: CountryId,
    candidate_gdp: RealGdp,
    damping: f64,
) {
    let a = damping.clamp(0.0, 1.0);
    let current = state.gdp.get_mut(country.index()).unwrap();
    current.0 += a * (candidate_gdp.0 - current.0);
}
