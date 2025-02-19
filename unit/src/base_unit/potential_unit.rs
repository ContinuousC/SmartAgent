/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{Prefix, SiPrefix};
use super::base_units::POTENTIAL_UNITS;
use super::BaseUnit;
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
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
pub enum PotentialUnit {
    Volt(SiPrefix),
}

impl BaseUnit for PotentialUnit {
    const LIST: &[Self] = &POTENTIAL_UNITS;
    const REFERENCE: Self = PotentialUnit::Volt(SiPrefix::Unit);
    fn multiplier(&self) -> f64 {
        match self {
            PotentialUnit::Volt(p) => p.multiplier(),
        }
    }
    fn scale(&self) -> Vec<Self> {
        match self {
            PotentialUnit::Volt(_) => {
                SiPrefix::SCALE.iter().map(|p| Self::Volt(*p)).collect()
            }
        }
    }
}

impl Display for PotentialUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            PotentialUnit::Volt(m) => write!(f, "{}V", m),
        }
    }
}
