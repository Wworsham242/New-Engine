#![forbid(unsafe_code)]

pub mod metadata;

use sim_contracts::DemographicsOutputs;
use sim_state::DemographicsState;
use sim_types::CountryId;

pub fn advance_month(_state: &mut DemographicsState) {
    // Phase 003 intentionally preserves population unchanged.
    // Cohorts, births, deaths, and migration arrive in a later vertical slice.
}

pub fn publish(state: &DemographicsState, country: CountryId) -> DemographicsOutputs {
    DemographicsOutputs {
        population: *state.population.get(country.index()).unwrap(),
    }
}
