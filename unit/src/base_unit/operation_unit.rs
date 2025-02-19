/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{DecPrefix, Prefix};
use super::base_units::OPERATIONS_UNITS;
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
pub enum OperationUnit {
    Operation(DecPrefix),
}

impl BaseUnit for OperationUnit {
    const LIST: &[Self] = &OPERATIONS_UNITS;
    const REFERENCE: Self = OperationUnit::Operation(DecPrefix::Unit);
    fn multiplier(&self) -> f64 {
        match self {
            OperationUnit::Operation(p) => p.multiplier(),
        }
    }
    fn scale(&self) -> Vec<Self> {
        match self {
            OperationUnit::Operation(_) => DecPrefix::SCALE
                .iter()
                .map(|p| Self::Operation(*p))
                .collect(),
        }
    }
}

impl Display for OperationUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            OperationUnit::Operation(m) => write!(f, "{}op", m),
        }
    }
}
