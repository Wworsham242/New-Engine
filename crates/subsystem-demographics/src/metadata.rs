use sim_equations::{VariableId, VariableMetadata};
use sim_types::{ProvenanceTag, ScopeKind, SubsystemId, TemporalSemantics, VariableKind};

pub const VAR_POPULATION: VariableId = VariableId(30);

pub static VARIABLES: &[VariableMetadata] = &[VariableMetadata {
    id: VAR_POPULATION,
    key: "demographics.population",
    owner: SubsystemId::Demographics,
    unit_key: "population",
    kind: VariableKind::Stock,
    temporal: TemporalSemantics::CurrentCommitted,
    scope: ScopeKind::Country,
    provenance: &[ProvenanceTag::OriginalDesign],
}];

pub static EQUATIONS: &[sim_equations::EquationMetadata] = &[];
