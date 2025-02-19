/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{Prefix, SiPrefix};
use super::base_units::LENGTH_UNITS;
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
pub enum LengthUnit {
    Meter(SiPrefix),
}

impl BaseUnit for LengthUnit {
    const LIST: &[Self] = &LENGTH_UNITS;
    const REFERENCE: Self = LengthUnit::Meter(SiPrefix::Unit);

    fn multiplier(&self) -> f64 {
        match self {
            LengthUnit::Meter(p) => p.multiplier(),
        }
    }

    fn scale(&self) -> Vec<Self> {
        match self {
            LengthUnit::Meter(_) => {
                SiPrefix::SCALE.iter().map(|p| Self::Meter(*p)).collect()
            }
        }
    }
}

impl Display for LengthUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            LengthUnit::Meter(m) => write!(f, "{}m", m),
        }
    }
}
