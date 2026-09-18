#![forbid(unsafe_code)]

use sim_state::{StateHash, WorldState};

#[derive(Clone, Copy, Debug)]
pub enum Shock {
    EnergyCapacityLossFraction(f64),
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
        // Phase 1: exogenous primitive input.
        let mut energy = world.energy.clone();
        let mut economy = world.economy.clone();

        if let Some(Shock::EnergyCapacityLossFraction(fraction)) = shock {
            subsystem_energy::apply_capacity_shock(&mut energy, fraction);
        }

        // Phase 2: bounded Jacobi-style coupled solve.
        let mut converged = false;
        let mut max_residual = f64::INFINITY;
        let mut iterations = 0;

        for iteration in 1..=self.solver.max_iterations {
            iterations = iteration;

            // Immutable contract snapshot C[n].
            let economy_to_energy = subsystem_economy::publish_to_energy(&economy);
            let energy_to_economy = subsystem_energy::publish(&energy);

            // Independent candidates from C[n].
            let energy_candidate = subsystem_energy::solve_candidate(&energy, economy_to_energy);
            let economy_candidate = subsystem_economy::solve_candidate(&economy, energy_to_economy);

            let gdp_scale = economy.gdp.0.abs().max(1.0);
            let price_scale = energy.price_index.0.abs().max(1.0);

            let gdp_residual = (economy_candidate.gdp.0 - economy.gdp.0).abs() / gdp_scale;
            let price_residual =
                (energy_candidate.price_index.0 - energy.price_index.0).abs() / price_scale;

            max_residual = gdp_residual.max(price_residual);

            energy = subsystem_energy::blend(&energy, &energy_candidate, self.solver.damping);
            economy = subsystem_economy::blend(&economy, &economy_candidate, self.solver.damping);

            if max_residual <= self.solver.tolerance {
                converged = true;
                break;
            }
        }

        // Phase 3: slower dependent systems consume the converged/bounded result.
        let economy_to_governance = subsystem_economy::publish_to_governance(&economy);
        let governance =
            subsystem_governance::solve_month(&world.governance, economy_to_governance);
        let demographics = subsystem_demographics::advance_month(&world.demographics);

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

    #[test]
    fn same_input_produces_same_hash() {
        let kernel = SimulationKernel::default();
        let mut a = WorldState::default();
        let mut b = WorldState::default();

        let ra = kernel
            .step(&mut a, Some(Shock::EnergyCapacityLossFraction(0.50)))
            .unwrap();
        let rb = kernel
            .step(&mut b, Some(Shock::EnergyCapacityLossFraction(0.50)))
            .unwrap();

        assert_eq!(ra.state_hash, rb.state_hash);
    }

    #[test]
    fn primitive_energy_shock_propagates_endogenously() {
        let kernel = SimulationKernel::default();
        let mut baseline = WorldState::default();
        let mut shock = baseline.clone();

        kernel.step(&mut baseline, None).unwrap();
        kernel
            .step(&mut shock, Some(Shock::EnergyCapacityLossFraction(0.50)))
            .unwrap();

        assert!(shock.energy.production.0 < baseline.energy.production.0);
        assert!(shock.energy.price_index.0 > baseline.energy.price_index.0);
        assert!(shock.economy.gdp.0 < baseline.economy.gdp.0);
        assert!(shock.governance.revenue.0 < baseline.governance.revenue.0);
    }
}
