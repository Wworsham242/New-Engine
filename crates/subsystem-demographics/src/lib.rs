#![forbid(unsafe_code)]

pub mod metadata;

use sim_contracts::DemographicsOutputs;
use sim_state::DemographicsState;

pub fn advance_month(current: &DemographicsState) -> DemographicsState {
    // Phase 001 intentionally preserves population unchanged.
    // Cohorts, births, deaths, and migration arrive in a later vertical slice.
    current.clone()
}

pub fn publish(state: &DemographicsState) -> DemographicsOutputs {
    DemographicsOutputs {
        population: state.population,
    }
}
