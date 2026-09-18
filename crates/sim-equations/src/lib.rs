#![forbid(unsafe_code)]

use sim_types::{Cadence, ProvenanceTag, ScopeKind, SubsystemId, TemporalSemantics, VariableKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EquationId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VariableId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SolverGroupId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EquationClass {
    AccountingIdentity,
    StockFlow,
    Behavioral,
    Capacity,
    Allocation,
    MarketAdjustment,
    Empirical,
    Lag,
    Transition,
    Constraint,
    Aggregation,
    SolverComponent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputSourceKind {
    LocalVariable,
    ContractField,
    Parameter,
    ExogenousInput,
    DelayedEvent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputSemantics {
    Set,
    AddFlow,
    SubtractFlow,
    Accumulate,
    ConstraintBound,
    SolverContribution,
}

#[derive(Clone, Copy, Debug)]
pub struct EquationInput {
    pub variable: VariableId,
    pub source: InputSourceKind,
    pub temporal: TemporalSemantics,
}

#[derive(Clone, Copy, Debug)]
pub struct EquationOutput {
    pub variable: VariableId,
    pub semantics: OutputSemantics,
}

#[derive(Clone, Debug)]
pub struct EquationMetadata {
    pub id: EquationId,
    pub key: &'static str,
    pub owner: SubsystemId,
    pub class: EquationClass,
    pub cadence: Cadence,
    pub inputs: &'static [EquationInput],
    pub outputs: &'static [EquationOutput],
    pub solver_group: Option<SolverGroupId>,
    pub provenance: &'static [ProvenanceTag],
    pub description: &'static str,
}

#[derive(Clone, Debug)]
pub struct VariableMetadata {
    pub id: VariableId,
    pub key: &'static str,
    pub owner: SubsystemId,
    pub unit_key: &'static str,
    pub kind: VariableKind,
    pub temporal: TemporalSemantics,
    pub scope: ScopeKind,
    pub provenance: &'static [ProvenanceTag],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SolverGroupKind {
    Local,
    OuterCoupled,
    ConstraintReconciliation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NonConvergencePolicy {
    Abort,
    CommitBestIterationWithDiagnostic,
}

#[derive(Clone, Copy, Debug)]
pub struct SolverPolicyMetadata {
    pub min_iterations: u32,
    pub max_iterations: u32,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
    pub damping: f64,
    pub failure_policy: NonConvergencePolicy,
}

#[derive(Clone, Debug)]
pub struct SolverGroupMetadata {
    pub id: SolverGroupId,
    pub key: &'static str,
    pub kind: SolverGroupKind,
    pub cadence: Cadence,
    pub convergence_variables: &'static [VariableId],
    pub policy: SolverPolicyMetadata,
    pub provenance: &'static [ProvenanceTag],
    pub description: &'static str,
}
