#![forbid(unsafe_code)]

pub mod metadata;

use sim_contracts::EconomyToGovernance;
use sim_state::GovernanceState;
use sim_units::Currency;

pub fn solve_month(current: &GovernanceState, economy: EconomyToGovernance) -> GovernanceState {
    GovernanceState {
        tax_rate: current.tax_rate,
        revenue: Currency(economy.taxable_output.0 * current.tax_rate.0),
        spending: current.spending,
    }
}
