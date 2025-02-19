/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::{base_units::FAN_SPEED_UNITS, BaseUnit};
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
pub enum FanSpeedUnit {
    RPM,
    RPS,
}

impl BaseUnit for FanSpeedUnit {
    const LIST: &[Self] = &FAN_SPEED_UNITS;
    const REFERENCE: Self = FanSpeedUnit::RPM;
    fn multiplier(&self) -> f64 {
        match self {
            FanSpeedUnit::RPM => 1.0,
            FanSpeedUnit::RPS => 60.0,
        }
    }
    fn scale(&self) -> Vec<Self> {
        vec![*self]
    }
}

impl Display for FanSpeedUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            FanSpeedUnit::RPM => write!(f, "rpm"),
            FanSpeedUnit::RPS => write!(f, "rps"),
        }
    }
}
