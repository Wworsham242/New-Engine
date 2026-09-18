#![forbid(unsafe_code)]

use sim_types::{Cadence, ProvenanceTag, SubsystemId};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EquationId(pub u32);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EquationClass {
    AccountingIdentity,
    StockFlow,
    Behavioral,
    Capacity,
    Allocation,
    MarketAdjustment,
    Lag,
    Constraint,
    Aggregation,
    SolverComponent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalReference {
    CurrentCommitted,
    CurrentIteration,
    PreviousTick,
    RollingAverage,
    PipelineState,
}

#[derive(Clone, Debug)]
pub struct EquationMetadata {
    pub id: EquationId,
    pub key: &'static str,
    pub owner: SubsystemId,
    pub class: EquationClass,
    pub cadence: Cadence,
    pub temporal_reference: TemporalReference,
    pub provenance: &'static [ProvenanceTag],
}
