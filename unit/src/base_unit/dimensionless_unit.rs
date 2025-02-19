/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{DecPrefix, Prefix};
use super::base_units::DIMENSIONLESS_UNITS;
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

pub enum DimensionlessUnit {
    Count(DecPrefix),
    Percent,
    Permille,
}

impl BaseUnit for DimensionlessUnit {
    const LIST: &[Self] = &DIMENSIONLESS_UNITS;
    const REFERENCE: Self = DimensionlessUnit::Count(DecPrefix::Unit);
    fn multiplier(&self) -> f64 {
        match self {
            DimensionlessUnit::Count(m) => m.multiplier(),
            DimensionlessUnit::Percent => 0.01,
            DimensionlessUnit::Permille => 0.001,
        }
    }
    fn normalize(&self) -> Self {
        match self {
            DimensionlessUnit::Count(_) => {
                DimensionlessUnit::Count(DecPrefix::Unit)
            }
            DimensionlessUnit::Percent => DimensionlessUnit::Percent,
            DimensionlessUnit::Permille => DimensionlessUnit::Permille,
        }
    }
    fn scale(&self) -> Vec<Self> {
        match self {
            DimensionlessUnit::Count(_) => DecPrefix::SCALE
                .iter()
                .map(|p| DimensionlessUnit::Count(*p))
                .collect(),
            DimensionlessUnit::Percent => vec![Self::Percent],
            DimensionlessUnit::Permille => vec![Self::Permille],
        }
    }
}

impl Display for DimensionlessUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            DimensionlessUnit::Count(m) => m.fmt(f),
            DimensionlessUnit::Percent => write!(f, "%"),
            DimensionlessUnit::Permille => write!(f, "‰"),
        }
    }
}
