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

        // ADR-005: physical time advances once per master tick, never once per
        // solver iteration. This temporal base remains immutable during solve.
        let energy_temporal_base = world.energy.clone();

        let mut energy = world.energy.clone();
        let mut economy = world.economy.clone();
        let country_count = world.country_count();

        let mut converged = false;
        let mut max_residual = f64::INFINITY;
        let mut iterations = 0;

        for iteration in 1..=self.solver.max_iterations {
            iterations = iteration;
            max_residual = 0.0;

            let economy_to_energy: Vec<_> = (0..country_count)
                .map(|index| {
                    subsystem_economy::publish_to_energy(&economy, CountryId(index as u32))
                })
                .collect();

            let energy_to_economy: Vec<_> = (0..country_count)
                .map(|index| subsystem_energy::publish(&energy, CountryId(index as u32)))
                .collect();

            let energy_candidate =
                subsystem_energy::solve_global_candidate(&energy_temporal_base, &economy_to_energy);

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

            for (index, economy_candidate) in economy_candidates.iter().enumerate() {
                let current_gdp = economy.gdp.get(index).unwrap().0;
                let current_price = energy.price_index.get(index).unwrap().0;
                let candidate_gdp = economy_candidate.0;
                let candidate_price = energy_candidate.price_index.get(index).unwrap().0;

                let gdp_residual = (candidate_gdp - current_gdp).abs() / current_gdp.abs().max(1.0);
                let price_residual =
                    (candidate_price - current_price).abs() / current_price.abs().max(1.0);

                max_residual = max_residual.max(gdp_residual.max(price_residual));
            }

            subsystem_energy::apply_candidate(&mut energy, &energy_candidate, self.solver.damping);

            for (index, economy_candidate) in economy_candidates.iter().copied().enumerate() {
                subsystem_economy::blend_country(
                    &mut economy,
                    CountryId(index as u32),
                    economy_candidate,
                    self.solver.damping,
                );
            }

            if max_residual <= self.solver.tolerance {
                converged = true;
                break;
            }
        }

        let mut governance = world.governance.clone();
        for index in 0..country_count {
            let country = CountryId(index as u32);
            let contract = subsystem_economy::publish_to_governance(&economy, country);
            subsystem_governance::solve_country_month(&mut governance, country, contract);
        }

        let mut demographics = world.demographics.clone();
        subsystem_demographics::advance_month(&mut demographics);

        world.energy = energy;
        world.economy = economy;
        world.governance = governance;
        world.demographics = demographics;
        world.tick = world.tick.next();

        world.validate()?;

        if subsystem_energy::trade_conservation_error(&world.energy) > 1.0e-12 {
            return Err("physical energy trade conservation failed");
        }

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
    fn same_delayed_input_produces_same_hash() {
        let kernel = SimulationKernel::default();
        let mut a = WorldState::demo();
        let mut b = WorldState::demo();

        let shock_a = usa_oil_shock(&a);
        let shock_b = usa_oil_shock(&b);

        for month in 0..4 {
            let sa = if month == 0 { Some(shock_a) } else { None };
            let sb = if month == 0 { Some(shock_b) } else { None };
            kernel.step(&mut a, sa).unwrap();
            kernel.step(&mut b, sb).unwrap();
        }

        assert_eq!(a.canonical_hash(), b.canonical_hash());
    }

    #[test]
    fn canadian_effect_is_delayed_by_transit_and_inventory_buffers() {
        let kernel = SimulationKernel::default();

        let mut baseline = WorldState::demo();
        let mut shock = baseline.clone();
        let shock_event = usa_oil_shock(&shock);

        let can = shock.country_id_by_key("CAN").unwrap();

        let mut baseline_can_gdp = Vec::new();
        let mut shock_can_gdp = Vec::new();

        for month in 0..4 {
            kernel.step(&mut baseline, None).unwrap();
            kernel
                .step(
                    &mut shock,
                    if month == 0 { Some(shock_event) } else { None },
                )
                .unwrap();

            baseline_can_gdp.push(*baseline.economy.gdp.get(can.index()).unwrap());
            shock_can_gdp.push(*shock.economy.gdp.get(can.index()).unwrap());
        }

        // Month 1: old shipments are already in transit.
        assert_eq!(shock_can_gdp[0].0, baseline_can_gdp[0].0);

        // Months 2-3: Canadian inventory absorbs the import shortfall.
        assert_eq!(shock_can_gdp[1].0, baseline_can_gdp[1].0);
        assert_eq!(shock_can_gdp[2].0, baseline_can_gdp[2].0);

        // Month 4: buffer is depleted and macro effects emerge.
        assert!(shock_can_gdp[3].0 < baseline_can_gdp[3].0);
    }

    #[test]
    fn solver_iterations_do_not_multiply_inventory_depletion() {
        let kernel = SimulationKernel::default();
        let mut shock = WorldState::demo();

        let usa = shock.country_id_by_key("USA").unwrap();
        let oil = shock.energy_type_id_by_key("oil").unwrap();

        // Use a deliberately small opening buffer so the primitive capacity
        // shock both draws inventory and leaves a residual shortage. That
        // guarantees the Economy<->Energy solver needs multiple numerical
        // iterations, which is necessary for this test to exercise the
        // ADR-005 invariant.
        shock
            .energy
            .inventory_by_type
            .get_mut(usa.index(), oil.0 as usize)
            .unwrap()
            .0 = 5.0;

        let event = usa_oil_shock(&shock);

        let before = shock
            .energy
            .inventory_by_type
            .get(usa.index(), oil.0 as usize)
            .unwrap()
            .0;

        let report = kernel.step(&mut shock, Some(event)).unwrap();

        let after = shock
            .energy
            .inventory_by_type
            .get(usa.index(), oil.0 as usize)
            .unwrap()
            .0;

        let draw = shock
            .energy
            .inventory_draw_by_type
            .get(usa.index(), oil.0 as usize)
            .unwrap()
            .0;

        assert!(report.iterations > 1);
        assert!(draw > 0.0);
        assert!(after >= -1.0e-12);
        assert!(draw <= before + 1.0e-12);
        assert!((before - draw - after).abs() <= 1.0e-12);
    }
}
