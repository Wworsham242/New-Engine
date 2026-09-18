use sim_equations::{
    NonConvergencePolicy, SolverGroupId, SolverGroupKind, SolverGroupMetadata,
    SolverPolicyMetadata, VariableId,
};
use sim_registry::ModelRegistry;
use sim_types::{Cadence, ProvenanceTag};

static CORE_CONVERGENCE_VARIABLES: &[VariableId] = &[VariableId(1), VariableId(13), VariableId(14)];

static SOLVER_GROUPS: &[SolverGroupMetadata] = &[SolverGroupMetadata {
    id: SolverGroupId(1),
    key: "core.energy_economy",
    kind: SolverGroupKind::OuterCoupled,
    cadence: Cadence::Monthly,
    convergence_variables: CORE_CONVERGENCE_VARIABLES,
    policy: SolverPolicyMetadata {
        min_iterations: 1,
        max_iterations: 64,
        absolute_tolerance: 1.0e-8,
        relative_tolerance: 1.0e-8,
        damping: 0.5,
        failure_policy: NonConvergencePolicy::CommitBestIterationWithDiagnostic,
    },
    provenance: &[ProvenanceTag::OriginalDesign],
    description: "Phase-002 registered Jacobi-style outer Energy/Economy coupling.",
}];

pub fn model_registry() -> ModelRegistry {
    static VARIABLES: std::sync::OnceLock<Vec<sim_equations::VariableMetadata>> =
        std::sync::OnceLock::new();
    static EQUATIONS: std::sync::OnceLock<Vec<sim_equations::EquationMetadata>> =
        std::sync::OnceLock::new();

    let variables = VARIABLES.get_or_init(|| {
        let mut out = Vec::new();
        out.extend_from_slice(subsystem_economy::metadata::VARIABLES);
        out.extend_from_slice(subsystem_energy::metadata::VARIABLES);
        out.extend_from_slice(subsystem_governance::metadata::VARIABLES);
        out.extend_from_slice(subsystem_demographics::metadata::VARIABLES);
        out.sort_by_key(|v| v.id);
        out
    });

    let equations = EQUATIONS.get_or_init(|| {
        let mut out = Vec::new();
        out.extend_from_slice(subsystem_economy::metadata::EQUATIONS);
        out.extend_from_slice(subsystem_energy::metadata::EQUATIONS);
        out.extend_from_slice(subsystem_governance::metadata::EQUATIONS);
        out.extend_from_slice(subsystem_demographics::metadata::EQUATIONS);
        out.sort_by_key(|e| e.id);
        out
    });

    ModelRegistry {
        variables,
        equations,
        solver_groups: SOLVER_GROUPS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_model_is_valid() {
        model_registry().validate().unwrap();
    }
}
