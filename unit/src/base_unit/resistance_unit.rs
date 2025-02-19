/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{Prefix, SiPrefix};
use super::base_units::RESISTANCE_UNITS;
use super::{BaseUnit, ConductivityUnit};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

#[derive(
    Serialize,
    Deserialize,
    PartialEq,
    PartialOrd,
    Eq,
    Ord,
    Hash,
    Clone,
    Copy,
    Debug,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum ResistanceUnit {
    Ohm(SiPrefix),
}

impl ResistanceUnit {
    pub fn to_conductivity(self) -> (f64, ConductivityUnit) {
        (self.multiplier().powi(-1), ConductivityUnit::REFERENCE)
    }
}

impl BaseUnit for ResistanceUnit {
    const LIST: &[Self] = &RESISTANCE_UNITS;
    const REFERENCE: Self = ResistanceUnit::Ohm(SiPrefix::Unit);
    fn multiplier(&self) -> f64 {
        match self {
            ResistanceUnit::Ohm(p) => p.multiplier(),
        }
    }
    fn scale(&self) -> Vec<Self> {
        match self {
            ResistanceUnit::Ohm(_) => {
                SiPrefix::SCALE.iter().map(|p| Self::Ohm(*p)).collect()
            }
        }
    }
}

impl Display for ResistanceUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            ResistanceUnit::Ohm(m) => write!(f, "{}Ω", m),
        }
    }
}
