#![forbid(unsafe_code)]

use sim_equations::{
    EquationId, EquationMetadata, SolverGroupId, SolverGroupMetadata, VariableId, VariableMetadata,
};
use sim_types::{CountryId, EnergyTypeId, RegionId, SectorId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountryRecord {
    pub id: CountryId,
    pub key: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionRecord {
    pub id: RegionId,
    pub key: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SectorRecord {
    pub id: SectorId,
    pub key: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnergyTypeRecord {
    pub id: EnergyTypeId,
    pub key: String,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct WorldRegistry {
    countries: Vec<CountryRecord>,
    regions: Vec<RegionRecord>,
    sectors: Vec<SectorRecord>,
    energy_types: Vec<EnergyTypeRecord>,
}

impl WorldRegistry {
    pub fn builder() -> WorldRegistryBuilder {
        WorldRegistryBuilder::default()
    }

    pub fn countries(&self) -> &[CountryRecord] {
        &self.countries
    }

    pub fn regions(&self) -> &[RegionRecord] {
        &self.regions
    }

    pub fn sectors(&self) -> &[SectorRecord] {
        &self.sectors
    }

    pub fn energy_types(&self) -> &[EnergyTypeRecord] {
        &self.energy_types
    }

    pub fn validate(&self) -> Result<(), RegistryError> {
        validate_unique_keys(self.countries.iter().map(|x| x.key.as_str()), "country")?;
        validate_unique_keys(self.regions.iter().map(|x| x.key.as_str()), "region")?;
        validate_unique_keys(self.sectors.iter().map(|x| x.key.as_str()), "sector")?;
        validate_unique_keys(
            self.energy_types.iter().map(|x| x.key.as_str()),
            "energy type",
        )?;

        for (expected, record) in self.countries.iter().enumerate() {
            if record.id.0 as usize != expected {
                return Err(RegistryError::NonCanonicalId("country"));
            }
        }
        for (expected, record) in self.regions.iter().enumerate() {
            if record.id.0 as usize != expected {
                return Err(RegistryError::NonCanonicalId("region"));
            }
        }
        for (expected, record) in self.sectors.iter().enumerate() {
            if record.id.0 as usize != expected {
                return Err(RegistryError::NonCanonicalId("sector"));
            }
        }
        for (expected, record) in self.energy_types.iter().enumerate() {
            if record.id.0 as usize != expected {
                return Err(RegistryError::NonCanonicalId("energy type"));
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
pub struct WorldRegistryBuilder {
    countries: Vec<(String, String)>,
    regions: Vec<(String, String)>,
    sectors: Vec<(String, String)>,
    energy_types: Vec<(String, String)>,
}

impl WorldRegistryBuilder {
    pub fn country(mut self, key: impl Into<String>, name: impl Into<String>) -> Self {
        self.countries.push((key.into(), name.into()));
        self
    }

    pub fn region(mut self, key: impl Into<String>, name: impl Into<String>) -> Self {
        self.regions.push((key.into(), name.into()));
        self
    }

    pub fn sector(mut self, key: impl Into<String>, name: impl Into<String>) -> Self {
        self.sectors.push((key.into(), name.into()));
        self
    }

    pub fn energy_type(mut self, key: impl Into<String>, name: impl Into<String>) -> Self {
        self.energy_types.push((key.into(), name.into()));
        self
    }

    pub fn build(self) -> Result<WorldRegistry, RegistryError> {
        let registry = WorldRegistry {
            countries: self
                .countries
                .into_iter()
                .enumerate()
                .map(|(i, (key, name))| CountryRecord {
                    id: CountryId(i as u32),
                    key,
                    name,
                })
                .collect(),
            regions: self
                .regions
                .into_iter()
                .enumerate()
                .map(|(i, (key, name))| RegionRecord {
                    id: RegionId(i as u32),
                    key,
                    name,
                })
                .collect(),
            sectors: self
                .sectors
                .into_iter()
                .enumerate()
                .map(|(i, (key, name))| SectorRecord {
                    id: SectorId(i as u16),
                    key,
                    name,
                })
                .collect(),
            energy_types: self
                .energy_types
                .into_iter()
                .enumerate()
                .map(|(i, (key, name))| EnergyTypeRecord {
                    id: EnergyTypeId(i as u16),
                    key,
                    name,
                })
                .collect(),
        };

        registry.validate()?;
        Ok(registry)
    }
}

#[derive(Clone, Debug)]
pub struct ModelRegistry {
    pub variables: &'static [VariableMetadata],
    pub equations: &'static [EquationMetadata],
    pub solver_groups: &'static [SolverGroupMetadata],
}

impl ModelRegistry {
    pub fn validate(&self) -> Result<(), RegistryError> {
        validate_unique_copy(self.variables.iter().map(|x| x.id), "variable id")?;
        validate_unique_keys(self.variables.iter().map(|x| x.key), "variable key")?;

        validate_unique_copy(self.equations.iter().map(|x| x.id), "equation id")?;
        validate_unique_keys(self.equations.iter().map(|x| x.key), "equation key")?;

        validate_unique_copy(self.solver_groups.iter().map(|x| x.id), "solver group id")?;
        validate_unique_keys(self.solver_groups.iter().map(|x| x.key), "solver group key")?;

        for equation in self.equations {
            if let Some(group_id) = equation.solver_group
                && !self.solver_groups.iter().any(|g| g.id == group_id)
            {
                return Err(RegistryError::UnknownSolverGroup {
                    equation: equation.id,
                    group: group_id,
                });
            }

            for input in equation.inputs {
                if !self.variables.iter().any(|v| v.id == input.variable) {
                    return Err(RegistryError::UnknownVariable {
                        equation: equation.id,
                        variable: input.variable,
                    });
                }
            }

            for output in equation.outputs {
                if !self.variables.iter().any(|v| v.id == output.variable) {
                    return Err(RegistryError::UnknownVariable {
                        equation: equation.id,
                        variable: output.variable,
                    });
                }
            }
        }

        for group in self.solver_groups {
            if group.policy.max_iterations == 0 {
                return Err(RegistryError::InvalidSolverPolicy(group.id));
            }
            if group.policy.min_iterations > group.policy.max_iterations {
                return Err(RegistryError::InvalidSolverPolicy(group.id));
            }
            if !(0.0..=1.0).contains(&group.policy.damping) {
                return Err(RegistryError::InvalidSolverPolicy(group.id));
            }
            for variable in group.convergence_variables {
                if !self.variables.iter().any(|v| v.id == *variable) {
                    return Err(RegistryError::UnknownConvergenceVariable {
                        group: group.id,
                        variable: *variable,
                    });
                }
            }
        }

        // Enforce one normal authoritative writer per variable in Phase 002.
        for variable in self.variables {
            let writers = self
                .equations
                .iter()
                .flat_map(|equation| {
                    equation
                        .outputs
                        .iter()
                        .map(move |output| (equation.id, output))
                })
                .filter(|(_, output)| output.variable == variable.id)
                .count();

            if writers > 1 {
                return Err(RegistryError::MultipleWriters(variable.id));
            }
        }

        Ok(())
    }

    pub fn variable(&self, id: VariableId) -> Option<&VariableMetadata> {
        self.variables.iter().find(|x| x.id == id)
    }

    pub fn equation(&self, id: EquationId) -> Option<&EquationMetadata> {
        self.equations.iter().find(|x| x.id == id)
    }

    pub fn solver_group(&self, id: SolverGroupId) -> Option<&SolverGroupMetadata> {
        self.solver_groups.iter().find(|x| x.id == id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    DuplicateKey(&'static str),
    DuplicateId(&'static str),
    NonCanonicalId(&'static str),
    UnknownVariable {
        equation: EquationId,
        variable: VariableId,
    },
    UnknownSolverGroup {
        equation: EquationId,
        group: SolverGroupId,
    },
    UnknownConvergenceVariable {
        group: SolverGroupId,
        variable: VariableId,
    },
    MultipleWriters(VariableId),
    InvalidSolverPolicy(SolverGroupId),
}

fn validate_unique_keys<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &'static str,
) -> Result<(), RegistryError> {
    let mut ordered: Vec<&str> = values.into_iter().collect();
    ordered.sort_unstable();
    if ordered.windows(2).any(|w| w[0] == w[1]) {
        return Err(RegistryError::DuplicateKey(label));
    }
    Ok(())
}

fn validate_unique_copy<T: Copy + Ord>(
    values: impl IntoIterator<Item = T>,
    label: &'static str,
) -> Result<(), RegistryError> {
    let mut ordered: Vec<T> = values.into_iter().collect();
    ordered.sort_unstable();
    if ordered.windows(2).any(|w| w[0] == w[1]) {
        return Err(RegistryError::DuplicateId(label));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn world_registry_assigns_canonical_ids() {
        let world = WorldRegistry::builder()
            .country("USA", "United States")
            .country("CAN", "Canada")
            .sector("industry", "Industry")
            .energy_type("oil", "Oil")
            .build()
            .unwrap();

        assert_eq!(world.countries()[0].id, CountryId(0));
        assert_eq!(world.countries()[1].id, CountryId(1));
        assert_eq!(world.sectors()[0].id, SectorId(0));
        assert_eq!(world.energy_types()[0].id, EnergyTypeId(0));
    }

    #[test]
    fn duplicate_world_keys_fail() {
        let result = WorldRegistry::builder()
            .country("USA", "United States")
            .country("USA", "Duplicate")
            .build();

        assert!(matches!(
            result,
            Err(RegistryError::DuplicateKey("country"))
        ));
    }
}
