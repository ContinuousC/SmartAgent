/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::{base_units::TEMPERATURE_UNITS, BaseUnit};
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
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
    Kelvin,
}

impl BaseUnit for TemperatureUnit {
    const LIST: &[Self] = &TEMPERATURE_UNITS;
    const REFERENCE: Self = TemperatureUnit::Kelvin;
    fn multiplier(&self) -> f64 {
        match self {
            TemperatureUnit::Kelvin => 1.0,
            TemperatureUnit::Celsius => 1.0,
            TemperatureUnit::Fahrenheit => 5.0 / 9.0,
        }
    }
    fn offset(&self) -> f64 {
        match self {
            TemperatureUnit::Celsius => 273.15,
            TemperatureUnit::Fahrenheit => 459.67,
            TemperatureUnit::Kelvin => 0.0,
        }
    }
    fn normalize(&self) -> Self {
        Self::Celsius
    }
    fn scale(&self) -> Vec<Self> {
        vec![*self]
    }
}

impl Display for TemperatureUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            TemperatureUnit::Kelvin => write!(f, "K"),
            TemperatureUnit::Celsius => write!(f, "°C"),
            TemperatureUnit::Fahrenheit => write!(f, "°F"),
        }
    }
}
