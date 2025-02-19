/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{Prefix, SiPrefix};
use super::base_units::CURRENT_UNITS;
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
pub enum CurrentUnit {
    Ampere(SiPrefix),
}

impl BaseUnit for CurrentUnit {
    const LIST: &[Self] = &CURRENT_UNITS;
    const REFERENCE: Self = CurrentUnit::Ampere(SiPrefix::Unit);
    fn multiplier(&self) -> f64 {
        match self {
            CurrentUnit::Ampere(p) => p.multiplier(),
        }
    }
    fn scale(&self) -> Vec<Self> {
        SiPrefix::SCALE.iter().map(|p| Self::Ampere(*p)).collect()
    }
}

impl Display for CurrentUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            CurrentUnit::Ampere(m) => write!(f, "{}A", m),
        }
    }
}
