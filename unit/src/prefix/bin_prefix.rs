/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::prefix_trait::Prefix;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

/// Binary (base 1024) prefixes
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
pub enum BinPrefix {
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

impl Prefix for BinPrefix {
    const BASE: u64 = 1024;
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
            i64::MIN..=0 => (n, BinPrefix::Unit),
            1 => (n - 1, BinPrefix::Kilo),
            2 => (n - 2, BinPrefix::Mega),
            3 => (n - 3, BinPrefix::Giga),
            4 => (n - 4, BinPrefix::Tera),
            5 => (n - 5, BinPrefix::Peta),
            6 => (n - 6, BinPrefix::Exa),
            7 => (n - 7, BinPrefix::Zetta),
            8..=i64::MAX => (n - 8, BinPrefix::Yotta),
        }
    }

    fn power(&self) -> i64 {
        match self {
            BinPrefix::Unit => 0,
            BinPrefix::Kilo => 1,
            BinPrefix::Mega => 2,
            BinPrefix::Giga => 3,
            BinPrefix::Tera => 4,
            BinPrefix::Peta => 5,
            BinPrefix::Exa => 6,
            BinPrefix::Zetta => 7,
            BinPrefix::Yotta => 8,
        }
    }

    fn prefix(&self) -> &'static str {
        match self {
            BinPrefix::Unit => "",
            BinPrefix::Kilo => "k",
            BinPrefix::Mega => "M",
            BinPrefix::Giga => "G",
            BinPrefix::Tera => "T",
            BinPrefix::Peta => "P",
            BinPrefix::Exa => "E",
            BinPrefix::Zetta => "Z",
            BinPrefix::Yotta => "Y",
        }
    }
}

impl Display for BinPrefix {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.prefix())
    }
}
