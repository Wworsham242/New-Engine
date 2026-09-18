#![forbid(unsafe_code)]

use sim_kernel::{Shock, SimulationKernel};
use sim_state::WorldState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let initial = WorldState::default();
    let kernel = SimulationKernel::default();

    let mut baseline = initial.clone();
    let baseline_report = kernel.step(&mut baseline, None)?;

    let mut shock = initial;
    let shock_report = kernel.step(&mut shock, Some(Shock::EnergyCapacityLossFraction(0.50)))?;

    println!("New Engine Phase 001 deterministic vertical slice");
    println!();
    println!(
        "baseline: tick={} energy_capacity={:.3} production={:.3} price={:.6} gdp={:.3} revenue={:.3}",
        baseline.tick.0,
        baseline.energy.capacity.0,
        baseline.energy.production.0,
        baseline.energy.price_index.0,
        baseline.economy.gdp.0,
        baseline.governance.revenue.0,
    );
    println!(
        "baseline solver: iterations={} converged={} residual={:.3e}",
        baseline_report.iterations, baseline_report.converged, baseline_report.max_residual,
    );
    println!("baseline hash: {}", baseline_report.state_hash.to_hex());
    println!();
    println!(
        "shock:    tick={} energy_capacity={:.3} production={:.3} price={:.6} gdp={:.3} revenue={:.3}",
        shock.tick.0,
        shock.energy.capacity.0,
        shock.energy.production.0,
        shock.energy.price_index.0,
        shock.economy.gdp.0,
        shock.governance.revenue.0,
    );
    println!(
        "shock solver: iterations={} converged={} residual={:.3e}",
        shock_report.iterations, shock_report.converged, shock_report.max_residual,
    );
    println!("shock hash: {}", shock_report.state_hash.to_hex());

    Ok(())
}
