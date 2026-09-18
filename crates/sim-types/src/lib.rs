#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Tick(pub u64);

impl Tick {
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct CountryId(pub u32);

impl CountryId {
    pub const fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RegionId(pub u32);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ProvinceId(pub u32);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct SectorId(pub u16);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct EnergyTypeId(pub u16);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[repr(u16)]
pub enum SubsystemId {
    Demographics = 1,
    Economy = 2,
    Energy = 3,
    Governance = 4,
    Military = 5,
    Infrastructure = 6,
    Agriculture = 7,
    Environment = 8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvenanceTag {
    OriginalDesign,
    IfsObservedArchitecture,
    PublicLiterature,
    DatasetDerived,
    MilitaryReferenceModel,
    PerformanceSimplification,
    GameplayAbstraction,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cadence {
    Monthly,
    Quarterly,
    Annual,
    InternalSubstep,
    OnEvent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VariableKind {
    Stock,
    Flow,
    Rate,
    Index,
    Parameter,
    Derived,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalSemantics {
    CurrentCommitted,
    CurrentIteration,
    PreviousTick,
    RollingAverage,
    RollingSum,
    Trend,
    PipelineState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScopeKind {
    Global,
    Country,
    Region,
    Province,
    CountrySector,
    CountryEnergyType,
    CountryPairEnergyType,
}
