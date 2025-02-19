/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{Prefix, SiPrefix};
use super::base_units::FREQUENCY_UNITS;
use super::{BaseUnit, TimeUnit};
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
pub enum FrequencyUnit {
    Hertz(SiPrefix),
    PerTime(TimeUnit),
}

impl FrequencyUnit {
    pub fn to_time(self) -> (f64, TimeUnit) {
        (self.multiplier().powi(-1), TimeUnit::REFERENCE)
    }
}

impl BaseUnit for FrequencyUnit {
    const LIST: &[Self] = &FREQUENCY_UNITS;
    const REFERENCE: Self = FrequencyUnit::Hertz(SiPrefix::Unit);
    fn multiplier(&self) -> f64 {
        match self {
            FrequencyUnit::Hertz(p) => p.multiplier(),
            FrequencyUnit::PerTime(t) => 1.0 / t.multiplier(),
        }
    }
    fn scale(&self) -> Vec<Self> {
        match self {
            FrequencyUnit::Hertz(_) => {
                SiPrefix::SCALE.iter().map(|p| Self::Hertz(*p)).collect()
            }
            FrequencyUnit::PerTime(u) => {
                u.scale().iter().rev().map(|u| Self::PerTime(*u)).collect()
            }
        }
    }
}

impl Display for FrequencyUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            FrequencyUnit::Hertz(m) => write!(f, "{}Hz", m),
            FrequencyUnit::PerTime(t) => write!(f, "/{}", t),
        }
    }
}
