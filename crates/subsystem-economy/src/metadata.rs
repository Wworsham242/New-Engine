use sim_equations::{
    EquationClass, EquationId, EquationInput, EquationMetadata, EquationOutput, InputSourceKind,
    OutputSemantics, SolverGroupId, VariableId, VariableMetadata,
};
use sim_types::{Cadence, ProvenanceTag, ScopeKind, SubsystemId, TemporalSemantics, VariableKind};

pub const VAR_GDP: VariableId = VariableId(1);
pub const VAR_POTENTIAL_GDP: VariableId = VariableId(2);
pub const VAR_INVESTMENT: VariableId = VariableId(3);

pub static VARIABLES: &[VariableMetadata] = &[
    VariableMetadata {
        id: VAR_GDP,
        key: "economy.gdp",
        owner: SubsystemId::Economy,
        unit_key: "real_gdp",
        kind: VariableKind::Flow,
        temporal: TemporalSemantics::CurrentIteration,
        scope: ScopeKind::Global,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_POTENTIAL_GDP,
        key: "economy.potential_gdp",
        owner: SubsystemId::Economy,
        unit_key: "real_gdp",
        kind: VariableKind::Derived,
        temporal: TemporalSemantics::CurrentCommitted,
        scope: ScopeKind::Global,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_INVESTMENT,
        key: "economy.investment",
        owner: SubsystemId::Economy,
        unit_key: "currency",
        kind: VariableKind::Flow,
        temporal: TemporalSemantics::CurrentCommitted,
        scope: ScopeKind::Global,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
];

static GDP_INPUTS: &[EquationInput] = &[
    EquationInput {
        variable: VAR_POTENTIAL_GDP,
        source: InputSourceKind::LocalVariable,
        temporal: TemporalSemantics::CurrentCommitted,
    },
    EquationInput {
        variable: VariableId(13), // energy.shortage_fraction
        source: InputSourceKind::ContractField,
        temporal: TemporalSemantics::CurrentIteration,
    },
];

static GDP_OUTPUTS: &[EquationOutput] = &[EquationOutput {
    variable: VAR_GDP,
    semantics: OutputSemantics::Set,
}];

pub static EQUATIONS: &[EquationMetadata] = &[EquationMetadata {
    id: EquationId(1001),
    key: "economy.phase001_gdp_from_energy_availability",
    owner: SubsystemId::Economy,
    class: EquationClass::SolverComponent,
    cadence: Cadence::Monthly,
    inputs: GDP_INPUTS,
    outputs: GDP_OUTPUTS,
    solver_group: Some(SolverGroupId(1)),
    provenance: &[
        ProvenanceTag::OriginalDesign,
        ProvenanceTag::PerformanceSimplification,
    ],
    description: "Phase-001 uncalibrated scaffold equation proving Energy<->Economy coupling; not an IFs production equation.",
}];
