#![forbid(unsafe_code)]

use sim_units::{Currency, EnergyQuantity, Population, PriceIndex, Rate, RealGdp};

#[derive(Clone, Copy, Debug, Default)]
pub struct EconomyToEnergy {
    pub gdp: RealGdp,
    pub investment: Currency,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EnergyToEconomy {
    pub production: EnergyQuantity,
    pub demand: EnergyQuantity,
    pub price_index: PriceIndex,
    pub shortage_fraction: Rate,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EconomyToGovernance {
    pub taxable_output: RealGdp,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DemographicsOutputs {
    pub population: Population,
}
