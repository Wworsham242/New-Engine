#![forbid(unsafe_code)]

use sim_kernel::{Shock, SimulationKernel};
use sim_state::WorldState;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.as_slice() {
        [a, b] if a == "model" && b == "validate" => model_validate(),
        [a, b] if a == "equations" && b == "list" => equations_list(),
        [a, b] if a == "variables" && b == "list" => variables_list(),
        [a, b] if a == "solvers" && b == "list" => solvers_list(),
        [] => world_demo(),
        _ => {
            eprintln!("Usage:");
            eprintln!("  cargo run -p sim-tools");
            eprintln!("  cargo run -p sim-tools -- model validate");
            eprintln!("  cargo run -p sim-tools -- equations list");
            eprintln!("  cargo run -p sim-tools -- variables list");
            eprintln!("  cargo run -p sim-tools -- solvers list");
            std::process::exit(2);
        }
    }
}

fn model_validate() -> Result<(), Box<dyn std::error::Error>> {
    let registry = sim_kernel::registry::model_registry();
    registry
        .validate()
        .map_err(|err| format!("model registry validation failed: {err:?}"))?;

    println!(
        "model registry valid: variables={} equations={} solver_groups={}",
        registry.variables.len(),
        registry.equations.len(),
        registry.solver_groups.len(),
    );
    Ok(())
}

fn variables_list() -> Result<(), Box<dyn std::error::Error>> {
    let registry = sim_kernel::registry::model_registry();
    registry
        .validate()
        .map_err(|err| format!("model registry validation failed: {err:?}"))?;

    for variable in registry.variables {
        println!(
            "{:>4}  {:<40} owner={:?} unit={} kind={:?} scope={:?}",
            variable.id.0,
            variable.key,
            variable.owner,
            variable.unit_key,
            variable.kind,
            variable.scope,
        );
    }
    Ok(())
}

fn equations_list() -> Result<(), Box<dyn std::error::Error>> {
    let registry = sim_kernel::registry::model_registry();
    registry
        .validate()
        .map_err(|err| format!("model registry validation failed: {err:?}"))?;

    for equation in registry.equations {
        println!(
            "{:>4}  {:<52} owner={:?} class={:?} group={:?}",
            equation.id.0,
            equation.key,
            equation.owner,
            equation.class,
            equation.solver_group.map(|x| x.0),
        );
        println!("      {}", equation.description);
    }
    Ok(())
}

fn solvers_list() -> Result<(), Box<dyn std::error::Error>> {
    let registry = sim_kernel::registry::model_registry();
    registry
        .validate()
        .map_err(|err| format!("model registry validation failed: {err:?}"))?;

    for group in registry.solver_groups {
        println!(
            "{:>4}  {:<32} kind={:?} max_iter={} damping={} abs_tol={:.3e} rel_tol={:.3e}",
            group.id.0,
            group.key,
            group.kind,
            group.policy.max_iterations,
            group.policy.damping,
            group.policy.absolute_tolerance,
            group.policy.relative_tolerance,
        );
    }
    Ok(())
}

fn world_demo() -> Result<(), Box<dyn std::error::Error>> {
    let registry = sim_kernel::registry::model_registry();
    registry
        .validate()
        .map_err(|err| format!("model registry validation failed: {err:?}"))?;

    let initial = WorldState::demo();
    initial.validate()?;

    let usa = initial.country_id_by_key("USA").unwrap();
    let oil = initial.energy_type_id_by_key("oil").unwrap();

    let kernel = SimulationKernel::default();

    let mut baseline = initial.clone();
    let baseline_report = kernel.step(&mut baseline, None)?;

    let mut shock = initial;
    let shock_report = kernel.step(
        &mut shock,
        Some(Shock::EnergyCapacityLossFraction {
            country: usa,
            energy_type: oil,
            fraction: 0.50,
        }),
    )?;

    println!("New Engine Phase 003 country-indexed deterministic world slice");
    println!(
        "world: countries={} energy_types={} variables={} equations={} solver_groups={}",
        baseline.country_count(),
        baseline.energy_type_count(),
        registry.variables.len(),
        registry.equations.len(),
        registry.solver_groups.len(),
    );
    println!();

    for country in baseline.registry.countries() {
        let i = country.id.index();
        println!(
            "{} baseline: gdp={:.3} energy_price={:.6} shortage={:.6} revenue={:.3}",
            country.key,
            baseline.economy.gdp.get(i).unwrap().0,
            baseline.energy.price_index.get(i).unwrap().0,
            baseline.energy.shortage_fraction.get(i).unwrap().0,
            baseline.governance.revenue.get(i).unwrap().0,
        );
        println!(
            "{} shock:    gdp={:.3} energy_price={:.6} shortage={:.6} revenue={:.3}",
            country.key,
            shock.economy.gdp.get(i).unwrap().0,
            shock.energy.price_index.get(i).unwrap().0,
            shock.energy.shortage_fraction.get(i).unwrap().0,
            shock.governance.revenue.get(i).unwrap().0,
        );
        println!();
    }

    println!(
        "baseline solver: iterations={} converged={} residual={:.3e}",
        baseline_report.iterations, baseline_report.converged, baseline_report.max_residual,
    );
    println!("baseline hash: {}", baseline_report.state_hash.to_hex());
    println!(
        "shock solver: iterations={} converged={} residual={:.3e}",
        shock_report.iterations, shock_report.converged, shock_report.max_residual,
    );
    println!("shock hash: {}", shock_report.state_hash.to_hex());

    Ok(())
}
