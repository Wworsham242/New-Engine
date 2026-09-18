#![forbid(unsafe_code)]

pub mod metadata;

use sim_contracts::EconomyToGovernance;
use sim_state::GovernanceState;
use sim_types::CountryId;
use sim_units::Currency;

pub fn solve_country_month(
    state: &mut GovernanceState,
    country: CountryId,
    economy: EconomyToGovernance,
) {
    let tax_rate = state.tax_rate.get(country.index()).unwrap().0;
    *state.revenue.get_mut(country.index()).unwrap() =
        Currency(economy.taxable_output.0 * tax_rate);
}
