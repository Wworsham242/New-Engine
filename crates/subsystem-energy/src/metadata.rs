use sim_equations::{
    EquationClass, EquationId, EquationInput, EquationMetadata, EquationOutput, InputSourceKind,
    OutputSemantics, SolverGroupId, VariableId, VariableMetadata,
};
use sim_types::{Cadence, ProvenanceTag, ScopeKind, SubsystemId, TemporalSemantics, VariableKind};

pub const VAR_CAPACITY: VariableId = VariableId(10);
pub const VAR_PRODUCTION: VariableId = VariableId(11);
pub const VAR_DEMAND: VariableId = VariableId(12);
pub const VAR_SHORTAGE: VariableId = VariableId(13);
pub const VAR_PRICE: VariableId = VariableId(14);
pub const VAR_PLANNED_TRADE: VariableId = VariableId(15);
pub const VAR_REALIZED_TRADE: VariableId = VariableId(16);

pub static VARIABLES: &[VariableMetadata] = &[
    VariableMetadata {
        id: VAR_CAPACITY,
        key: "energy.capacity",
        owner: SubsystemId::Energy,
        unit_key: "energy_quantity",
        kind: VariableKind::Stock,
        temporal: TemporalSemantics::CurrentCommitted,
        scope: ScopeKind::CountryEnergyType,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_PRODUCTION,
        key: "energy.production",
        owner: SubsystemId::Energy,
        unit_key: "energy_quantity",
        kind: VariableKind::Flow,
        temporal: TemporalSemantics::CurrentIteration,
        scope: ScopeKind::CountryEnergyType,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_DEMAND,
        key: "energy.demand",
        owner: SubsystemId::Energy,
        unit_key: "energy_quantity",
        kind: VariableKind::Flow,
        temporal: TemporalSemantics::CurrentIteration,
        scope: ScopeKind::Country,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_SHORTAGE,
        key: "energy.shortage_fraction",
        owner: SubsystemId::Energy,
        unit_key: "rate",
        kind: VariableKind::Rate,
        temporal: TemporalSemantics::CurrentIteration,
        scope: ScopeKind::Country,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_PRICE,
        key: "energy.price_index",
        owner: SubsystemId::Energy,
        unit_key: "price_index",
        kind: VariableKind::Index,
        temporal: TemporalSemantics::CurrentIteration,
        scope: ScopeKind::Country,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_PLANNED_TRADE,
        key: "energy.planned_trade",
        owner: SubsystemId::Energy,
        unit_key: "energy_quantity",
        kind: VariableKind::Flow,
        temporal: TemporalSemantics::CurrentCommitted,
        scope: ScopeKind::CountryPairEnergyType,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
    VariableMetadata {
        id: VAR_REALIZED_TRADE,
        key: "energy.realized_trade",
        owner: SubsystemId::Energy,
        unit_key: "energy_quantity",
        kind: VariableKind::Flow,
        temporal: TemporalSemantics::CurrentIteration,
        scope: ScopeKind::CountryPairEnergyType,
        provenance: &[ProvenanceTag::OriginalDesign],
    },
];

static ENERGY_INPUTS: &[EquationInput] = &[
    EquationInput {
        variable: VAR_CAPACITY,
        source: InputSourceKind::LocalVariable,
        temporal: TemporalSemantics::CurrentCommitted,
    },
    EquationInput {
        variable: VAR_PLANNED_TRADE,
        source: InputSourceKind::LocalVariable,
        temporal: TemporalSemantics::CurrentCommitted,
    },
    EquationInput {
        variable: VariableId(1), // economy.gdp
        source: InputSourceKind::ContractField,
        temporal: TemporalSemantics::CurrentIteration,
    },
];

static ENERGY_OUTPUTS: &[EquationOutput] = &[
    EquationOutput {
        variable: VAR_PRODUCTION,
        semantics: OutputSemantics::Set,
    },
    EquationOutput {
        variable: VAR_REALIZED_TRADE,
        semantics: OutputSemantics::Set,
    },
    EquationOutput {
        variable: VAR_DEMAND,
        semantics: OutputSemantics::Set,
    },
    EquationOutput {
        variable: VAR_SHORTAGE,
        semantics: OutputSemantics::Set,
    },
    EquationOutput {
        variable: VAR_PRICE,
        semantics: OutputSemantics::Set,
    },
];

pub static EQUATIONS: &[EquationMetadata] = &[EquationMetadata {
    id: EquationId(2001),
    key: "energy.phase004_balance_trade_and_price",
    owner: SubsystemId::Energy,
    class: EquationClass::SolverComponent,
    cadence: Cadence::Monthly,
    inputs: ENERGY_INPUTS,
    outputs: ENERGY_OUTPUTS,
    solver_group: Some(SolverGroupId(1)),
    provenance: &[
        ProvenanceTag::OriginalDesign,
        ProvenanceTag::PerformanceSimplification,
    ],
    description: "Phase-004 uncalibrated physical energy balance with conserved country-to-country trade; architecture demonstration only.",
}];
