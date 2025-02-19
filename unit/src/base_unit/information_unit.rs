/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::super::prefix::{BinPrefix, DecPrefix, Prefix};
use super::base_units::INFORMATION_UNITS;
use super::BaseUnit;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

/* Unambiguous scaling here would require us to use the IEC standard
 * of representing binary prefixes as Ki, Mi, ... In this case both
 * prefix scaled could be fused into one (since they are not used by
 * other units). */
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
pub enum InformationUnit {
    Bit(DecPrefix),
    Byte(BinPrefix),
}

impl BaseUnit for InformationUnit {
    const LIST: &[Self] = &INFORMATION_UNITS;
    const REFERENCE: Self = InformationUnit::Byte(BinPrefix::Unit);

    fn multiplier(&self) -> f64 {
        match self {
            InformationUnit::Bit(p) => p.multiplier() / 8.0,
            InformationUnit::Byte(p) => p.multiplier(),
        }
    }

    fn normalize(&self) -> Self {
        match self {
            InformationUnit::Bit(_) => InformationUnit::Bit(DecPrefix::Unit),
            InformationUnit::Byte(_) => InformationUnit::Byte(BinPrefix::Unit),
        }
    }
    fn scale(&self) -> Vec<Self> {
        match self {
            InformationUnit::Bit(_) => {
                DecPrefix::SCALE.iter().map(|p| Self::Bit(*p)).collect()
            }
            InformationUnit::Byte(_) => {
                BinPrefix::SCALE.iter().map(|p| Self::Byte(*p)).collect()
            }
        }
    }
}

impl Display for InformationUnit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            InformationUnit::Bit(m) => write!(f, "{}b", m),
            InformationUnit::Byte(m) => write!(f, "{}B", m),
        }
    }
}
