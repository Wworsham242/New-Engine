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
            "{:>4}  {:<40} owner={:?} unit={} kind={:?} scope={:?} temporal={:?}",
            variable.id.0,
            variable.key,
            variable.owner,
            variable.unit_key,
            variable.kind,
            variable.scope,
            variable.temporal,
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
            "{:>4}  {:<56} owner={:?} class={:?} group={:?}",
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
    let can = initial.country_id_by_key("CAN").unwrap();
    let oil = initial.energy_type_id_by_key("oil").unwrap();

    let kernel = SimulationKernel::default();

    let mut baseline = initial.clone();
    let mut shock = initial;

    println!("New Engine Phase 005 delayed energy-buffer transmission slice");
    println!(
        "world: countries={} energy_types={} variables={} equations={} solver_groups={}",
        baseline.country_count(),
        baseline.energy_type_count(),
        registry.variables.len(),
        registry.equations.len(),
        registry.solver_groups.len(),
    );
    println!();
    println!(
        "shock: USA oil capacity -50% at month 1; transit delay=1 month; Canadian oil inventory starts at {:.3}",
        shock
            .energy
            .inventory_by_type
            .get(can.index(), oil.0 as usize)
            .unwrap()
            .0
    );
    println!();

    for month in 1..=4 {
        let baseline_report = kernel.step(&mut baseline, None)?;

        let shock_report = kernel.step(
            &mut shock,
            if month == 1 {
                Some(Shock::EnergyCapacityLossFraction {
                    country: usa,
                    energy_type: oil,
                    fraction: 0.50,
                })
            } else {
                None
            },
        )?;

        let baseline_flow = baseline
            .energy
            .realized_trade_by_type
            .get(usa.index(), can.index(), oil.0 as usize)
            .unwrap()
            .0;
        let shock_flow = shock
            .energy
            .realized_trade_by_type
            .get(usa.index(), can.index(), oil.0 as usize)
            .unwrap()
            .0;

        let can_inventory = shock
            .energy
            .inventory_by_type
            .get(can.index(), oil.0 as usize)
            .unwrap()
            .0;
        let can_draw = shock
            .energy
            .inventory_draw_by_type
            .get(can.index(), oil.0 as usize)
            .unwrap()
            .0;

        println!("MONTH {month}");
        println!(
            "  USA->CAN oil launched: baseline={:.3} shock={:.3}",
            baseline_flow, shock_flow
        );
        println!(
            "  CAN buffer: inventory_end={:.3} draw={:.3}",
            can_inventory, can_draw
        );
        println!(
            "  CAN baseline: gdp={:.3} price={:.6} shortage={:.6}",
            baseline.economy.gdp.get(can.index()).unwrap().0,
            baseline.energy.price_index.get(can.index()).unwrap().0,
            baseline
                .energy
                .shortage_fraction
                .get(can.index())
                .unwrap()
                .0,
        );
        println!(
            "  CAN shock:    gdp={:.3} price={:.6} shortage={:.6}",
            shock.economy.gdp.get(can.index()).unwrap().0,
            shock.energy.price_index.get(can.index()).unwrap().0,
            shock.energy.shortage_fraction.get(can.index()).unwrap().0,
        );
        println!(
            "  solvers: baseline_iter={} shock_iter={} shock_converged={}",
            baseline_report.iterations, shock_report.iterations, shock_report.converged,
        );
        println!();
    }

    println!(
        "trade conservation error: baseline={:.3e} shock={:.3e}",
        subsystem_energy::trade_conservation_error(&baseline.energy),
        subsystem_energy::trade_conservation_error(&shock.energy),
    );
    println!("baseline hash: {}", baseline.canonical_hash().to_hex());
    println!("shock hash:    {}", shock.canonical_hash().to_hex());

    Ok(())
}
