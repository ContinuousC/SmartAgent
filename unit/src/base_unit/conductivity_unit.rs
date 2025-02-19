/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{Prefix, SiPrefix};
use super::base_units::CONDUCTIVITY_UNITS;
use super::{BaseUnit, ResistanceUnit};
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
pub enum ConductivityUnit {
    Siemens(SiPrefix),
}

impl ConductivityUnit {
    pub fn to_resistance(self) -> (f64, ResistanceUnit) {
        (self.multiplier().powi(-1), ResistanceUnit::REFERENCE)
    }
}

impl BaseUnit for ConductivityUnit {
    const LIST: &[Self] = &CONDUCTIVITY_UNITS;
    const REFERENCE: Self = ConductivityUnit::Siemens(SiPrefix::Unit);
    fn multiplier(&self) -> f64 {
        match self {
            ConductivityUnit::Siemens(p) => p.multiplier(),
        }
    }
    fn scale(&self) -> Vec<Self> {
        SiPrefix::SCALE.iter().map(|p| Self::Siemens(*p)).collect()
    }
}

impl Display for ConductivityUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            ConductivityUnit::Siemens(m) => write!(f, "{}S", m),
        }
    }
}
