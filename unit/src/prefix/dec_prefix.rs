/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::prefix_trait::Prefix;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

/// Decimal (base 1000) prefixes.
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
pub enum DecPrefix {
    Unit,
    Kilo,
    Mega,
    Giga,
    Tera,
    Peta,
    Exa,
    Zetta,
    Yotta,
}

impl Prefix for DecPrefix {
    const BASE: u64 = 1000;
    const SCALE: &'static [Self] = &[
        Self::Unit,
        Self::Kilo,
        Self::Mega,
        Self::Giga,
        Self::Tera,
        Self::Peta,
        Self::Exa,
        Self::Zetta,
        Self::Yotta,
    ];

    fn from_power(n: i64) -> (i64, Self) {
        match n {
            i64::MIN..=0 => (n, DecPrefix::Unit),
            1 => (n - 1, DecPrefix::Kilo),
            2 => (n - 2, DecPrefix::Mega),
            3 => (n - 3, DecPrefix::Giga),
            4 => (n - 4, DecPrefix::Tera),
            5 => (n - 5, DecPrefix::Peta),
            6 => (n - 6, DecPrefix::Exa),
            7 => (n - 7, DecPrefix::Zetta),
            8..=i64::MAX => (n - 8, DecPrefix::Yotta),
        }
    }

    fn power(&self) -> i64 {
        match self {
            DecPrefix::Unit => 0,
            DecPrefix::Kilo => 1,
            DecPrefix::Mega => 2,
            DecPrefix::Giga => 3,
            DecPrefix::Tera => 4,
            DecPrefix::Peta => 5,
            DecPrefix::Exa => 6,
            DecPrefix::Zetta => 7,
            DecPrefix::Yotta => 8,
        }
    }

    fn prefix(&self) -> &'static str {
        match self {
            DecPrefix::Unit => "",
            DecPrefix::Kilo => "k",
            DecPrefix::Mega => "M",
            DecPrefix::Giga => "G",
            DecPrefix::Tera => "T",
            DecPrefix::Peta => "P",
            DecPrefix::Exa => "E",
            DecPrefix::Zetta => "Z",
            DecPrefix::Yotta => "Y",
        }
    }
}

impl Display for DecPrefix {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.prefix())
    }
}
