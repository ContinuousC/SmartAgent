/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{Prefix, SiPrefix};
use super::base_units::POWER_UNITS;
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
pub enum PowerUnit {
    Watt(SiPrefix),
    DBmW,
}

impl BaseUnit for PowerUnit {
    const LIST: &[Self] = &POWER_UNITS;
    const REFERENCE: Self = PowerUnit::Watt(SiPrefix::Unit);
    fn multiplier(&self) -> f64 {
        match self {
            PowerUnit::Watt(p) => p.multiplier(),
            PowerUnit::DBmW => 0.001,
        }
    }
    fn linearize(&self, n: f64) -> f64 {
        match self {
            PowerUnit::DBmW => 10f64.powf(n / 10.0),
            _ => n,
        }
    }
    fn delinearize(&self, n: f64) -> f64 {
        match self {
            PowerUnit::DBmW => n.log10() * 10.0,
            _ => n,
        }
    }
    fn scale(&self) -> Vec<Self> {
        match self {
            PowerUnit::Watt(_) => {
                SiPrefix::SCALE.iter().map(|p| Self::Watt(*p)).collect()
            }
            _ => vec![*self],
        }
    }
}

impl Display for PowerUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            PowerUnit::Watt(m) => write!(f, "{}W", m),
            PowerUnit::DBmW => write!(f, "dBmW"),
        }
    }
}
