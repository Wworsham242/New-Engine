#![forbid(unsafe_code)]

pub mod registry;

use sim_state::{StateHash, WorldState};
use sim_types::{CountryId, EnergyTypeId};

#[derive(Clone, Copy, Debug)]
pub enum Shock {
    EnergyCapacityLossFraction {
        country: CountryId,
        energy_type: EnergyTypeId,
        fraction: f64,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct SolverPolicy {
    pub tolerance: f64,
    pub damping: f64,
    pub max_iterations: u32,
}

impl Default for SolverPolicy {
    fn default() -> Self {
        Self {
            tolerance: 1.0e-8,
            damping: 0.5,
            max_iterations: 64,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TickReport {
    pub iterations: u32,
    pub converged: bool,
    pub max_residual: f64,
    pub state_hash: StateHash,
}

#[derive(Clone, Debug, Default)]
pub struct SimulationKernel {
    pub solver: SolverPolicy,
}

impl SimulationKernel {
    pub fn step(
        &self,
        world: &mut WorldState,
        shock: Option<Shock>,
    ) -> Result<TickReport, &'static str> {
        world.validate()?;

        // Phase 1: primitive exogenous input.
        if let Some(Shock::EnergyCapacityLossFraction {
            country,
            energy_type,
            fraction,
        }) = shock
        {
            subsystem_energy::apply_capacity_shock(
                &mut world.energy,
                country,
                energy_type,
                fraction,
            );
        }

        // Work on owned subsystem working state. Committed world state is not
        // exposed across subsystem boundaries during the coupled solve.
        let mut energy = world.energy.clone();
        let mut economy = world.economy.clone();

        let country_count = world.country_count();

        // Phase 2: bounded Jacobi-style coupled solve.
        let mut converged = false;
        let mut max_residual = f64::INFINITY;
        let mut iterations = 0;

        for iteration in 1..=self.solver.max_iterations {
            iterations = iteration;
            max_residual = 0.0;

            // Immutable C[n] contract snapshots in canonical CountryId order.
            let economy_to_energy: Vec<_> = (0..country_count)
                .map(|index| {
                    subsystem_economy::publish_to_energy(&economy, CountryId(index as u32))
                })
                .collect();

            let energy_to_economy: Vec<_> = (0..country_count)
                .map(|index| subsystem_energy::publish(&energy, CountryId(index as u32)))
                .collect();

            // Independent country candidates from the same iteration snapshot.
            let energy_candidates: Vec<_> = (0..country_count)
                .map(|index| {
                    let country = CountryId(index as u32);
                    subsystem_energy::solve_country_candidate(
                        &energy,
                        country,
                        economy_to_energy[index],
                    )
                })
                .collect();

            let economy_candidates: Vec<_> = (0..country_count)
                .map(|index| {
                    let country = CountryId(index as u32);
                    subsystem_economy::solve_country_candidate(
                        &economy,
                        country,
                        energy_to_economy[index],
                    )
                })
                .collect();

            // Residuals are reduced deterministically in ascending CountryId.
            for index in 0..country_count {
                let current_gdp = economy.gdp.get(index).unwrap().0;
                let current_price = energy.price_index.get(index).unwrap().0;
                let candidate_gdp = economy_candidates[index].0;
                let candidate_price = energy_candidates[index].2.0;

                let gdp_residual = (candidate_gdp - current_gdp).abs() / current_gdp.abs().max(1.0);
                let price_residual =
                    (candidate_price - current_price).abs() / current_price.abs().max(1.0);

                max_residual = max_residual.max(gdp_residual.max(price_residual));
            }

            // Deterministic barrier then damping/apply in ascending CountryId.
            for index in 0..country_count {
                let country = CountryId(index as u32);
                let (production, demand, price, shortage) = &energy_candidates[index];

                subsystem_energy::blend_country(
                    &mut energy,
                    country,
                    production,
                    *demand,
                    *price,
                    *shortage,
                    self.solver.damping,
                );

                subsystem_economy::blend_country(
                    &mut economy,
                    country,
                    economy_candidates[index],
                    self.solver.damping,
                );
            }

            if max_residual <= self.solver.tolerance {
                converged = true;
                break;
            }
        }

        // Phase 3: slower dependent systems consume final bounded result.
        let mut governance = world.governance.clone();
        for index in 0..country_count {
            let country = CountryId(index as u32);
            let contract = subsystem_economy::publish_to_governance(&economy, country);
            subsystem_governance::solve_country_month(&mut governance, country, contract);
        }

        let mut demographics = world.demographics.clone();
        subsystem_demographics::advance_month(&mut demographics);

        // Phase 4: one logical authoritative commit.
        world.energy = energy;
        world.economy = economy;
        world.governance = governance;
        world.demographics = demographics;
        world.tick = world.tick.next();

        world.validate()?;
        let state_hash = world.canonical_hash();

        Ok(TickReport {
            iterations,
            converged,
            max_residual,
            state_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usa_oil_shock(world: &WorldState) -> Shock {
        Shock::EnergyCapacityLossFraction {
            country: world.country_id_by_key("USA").unwrap(),
            energy_type: world.energy_type_id_by_key("oil").unwrap(),
            fraction: 0.50,
        }
    }

    #[test]
    fn same_multi_country_input_produces_same_hash() {
        let kernel = SimulationKernel::default();
        let mut a = WorldState::demo();
        let mut b = WorldState::demo();

        let shock_a = usa_oil_shock(&a);
        let shock_b = usa_oil_shock(&b);

        let ra = kernel.step(&mut a, Some(shock_a)).unwrap();
        let rb = kernel.step(&mut b, Some(shock_b)).unwrap();

        assert_eq!(ra.state_hash, rb.state_hash);
    }

    #[test]
    fn country_specific_energy_shock_propagates_only_through_target_country_in_phase003() {
        let kernel = SimulationKernel::default();
        let mut baseline = WorldState::demo();
        let mut shock = baseline.clone();
        let shock_event = usa_oil_shock(&shock);

        kernel.step(&mut baseline, None).unwrap();
        kernel.step(&mut shock, Some(shock_event)).unwrap();

        let usa = shock.country_id_by_key("USA").unwrap().index();
        let can = shock.country_id_by_key("CAN").unwrap().index();

        assert!(
            shock.energy.price_index.get(usa).unwrap().0
                > baseline.energy.price_index.get(usa).unwrap().0
        );
        assert!(shock.economy.gdp.get(usa).unwrap().0 < baseline.economy.gdp.get(usa).unwrap().0);
        assert!(
            shock.governance.revenue.get(usa).unwrap().0
                < baseline.governance.revenue.get(usa).unwrap().0
        );

        // Cross-country trade is not modeled yet, so Canada is deliberately unchanged.
        assert_eq!(
            shock.economy.gdp.get(can).unwrap().0,
            baseline.economy.gdp.get(can).unwrap().0
        );
    }
}
