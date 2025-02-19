/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{Prefix, SiPrefix};
use super::base_units::MASS_UNITS;
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
pub enum MassUnit {
    Gram(SiPrefix),
}

impl BaseUnit for MassUnit {
    const LIST: &[Self] = &MASS_UNITS;
    const REFERENCE: Self = MassUnit::Gram(SiPrefix::Kilo);
    fn multiplier(&self) -> f64 {
        match self {
            MassUnit::Gram(p) => p.multiplier() / 1000.0,
        }
    }
    fn scale(&self) -> Vec<Self> {
        match self {
            MassUnit::Gram(_) => {
                SiPrefix::SCALE.iter().map(|p| Self::Gram(*p)).collect()
            }
        }
    }
}

impl Display for MassUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            MassUnit::Gram(m) => write!(f, "{}g", m),
        }
    }
}
