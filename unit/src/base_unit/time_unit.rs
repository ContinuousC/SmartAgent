/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{FracPrefix, Prefix};
use super::base_units::TIME_UNITS;
use super::{BaseUnit, FrequencyUnit};
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
pub enum TimeUnit {
    Second(FracPrefix),
    Minute,
    Hour,
    Day,
    Week,
}

impl TimeUnit {
    pub fn to_frequency(self) -> (f64, FrequencyUnit) {
        (self.multiplier().powi(-1), FrequencyUnit::REFERENCE)
    }
}

impl BaseUnit for TimeUnit {
    const LIST: &[Self] = &TIME_UNITS;
    const REFERENCE: Self = TimeUnit::Second(FracPrefix::Unit);
    fn multiplier(&self) -> f64 {
        match self {
            TimeUnit::Second(m) => m.multiplier(),
            TimeUnit::Minute => 60.0,
            TimeUnit::Hour => 3600.0,
            TimeUnit::Day => 3600.0 * 24.0,
            TimeUnit::Week => 3600.0 * 24.0 * 7.0,
        }
    }
    fn scale(&self) -> Vec<Self> {
        FracPrefix::SCALE
            .iter()
            .map(|p| Self::Second(*p))
            .chain(vec![Self::Minute, Self::Hour, Self::Day, Self::Week])
            .collect()
    }
}

impl Display for TimeUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            TimeUnit::Second(m) => write!(f, "{}s", m),
            TimeUnit::Minute => write!(f, "min"),
            TimeUnit::Hour => write!(f, "h"),
            TimeUnit::Day => write!(f, "day"),
            TimeUnit::Week => write!(f, "week"),
        }
    }
}
