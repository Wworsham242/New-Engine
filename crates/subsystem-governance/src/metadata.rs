use sim_equations::{
    EquationClass, EquationId, EquationInput, EquationMetadata, EquationOutput, InputSourceKind,
    OutputSemantics, VariableId, VariableMetadata,
};
use sim_types::{Cadence, ProvenanceTag, ScopeKind, SubsystemId, TemporalSemantics, VariableKind};

pub const VAR_TAX_RATE: VariableId = VariableId(20);
pub const VAR_REVENUE: VariableId = VariableId(21);
pub const VAR_SPENDING: VariableId = VariableId(22);

pub static VARIABLES: &[VariableMetadata] = &[
    VariableMetadata {
        id: VAR_TAX_RATE,
        key: "governance.tax_rate",
        owner: SubsystemId::Governance,
        unit_key: "rate",
        kind: VariableKind::Parameter,
        temporal: TemporalSemantics::CurrentCommitted,
        scope: ScopeKind::Country,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_REVENUE,
        key: "governance.revenue",
        owner: SubsystemId::Governance,
        unit_key: "currency",
        kind: VariableKind::Flow,
        temporal: TemporalSemantics::CurrentCommitted,
        scope: ScopeKind::Country,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_SPENDING,
        key: "governance.spending",
        owner: SubsystemId::Governance,
        unit_key: "currency",
        kind: VariableKind::Flow,
        temporal: TemporalSemantics::CurrentCommitted,
        scope: ScopeKind::Country,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
];

static REVENUE_INPUTS: &[EquationInput] = &[
    EquationInput {
        variable: VAR_TAX_RATE,
        source: InputSourceKind::LocalVariable,
        temporal: TemporalSemantics::CurrentCommitted,
    },
    EquationInput {
        variable: VariableId(1), // economy.gdp
        source: InputSourceKind::ContractField,
        temporal: TemporalSemantics::CurrentCommitted,
    },
];

static REVENUE_OUTPUTS: &[EquationOutput] = &[EquationOutput {
    variable: VAR_REVENUE,
    semantics: OutputSemantics::Set,
}];

pub static EQUATIONS: &[EquationMetadata] = &[EquationMetadata {
    id: EquationId(3001),
    key: "governance.phase001_tax_revenue",
    owner: SubsystemId::Governance,
    class: EquationClass::AccountingIdentity,
    cadence: Cadence::Monthly,
    inputs: REVENUE_INPUTS,
    outputs: REVENUE_OUTPUTS,
    solver_group: None,
    provenance: &[ProvenanceTag::OriginalDesign],
    description: "Phase-001 scaffold tax-revenue identity used after the coupled Energy/Economy solve.",
}];
