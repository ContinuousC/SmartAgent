/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::prefix_trait::Prefix;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

/// Fractional Prefixes (eg. for seconds).
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
pub enum FracPrefix {
    Yocto,
    Zepto,
    Atto,
    Femto,
    Pico,
    Nano,
    Micro,
    Milli,
    Unit,
}

impl Prefix for FracPrefix {
    const BASE: u64 = 1000;
    const SCALE: &'static [Self] = &[
        Self::Yocto,
        Self::Zepto,
        Self::Atto,
        Self::Femto,
        Self::Pico,
        Self::Nano,
        Self::Micro,
        Self::Milli,
        Self::Unit,
    ];

    fn from_power(n: i64) -> (i64, Self) {
        match n {
            i64::MIN..=-8 => (n + 8, FracPrefix::Yocto),
            -7 => (n + 7, FracPrefix::Zepto),
            -6 => (n + 6, FracPrefix::Atto),
            -5 => (n + 5, FracPrefix::Femto),
            -4 => (n + 4, FracPrefix::Pico),
            -3 => (n + 3, FracPrefix::Nano),
            -2 => (n + 2, FracPrefix::Micro),
            -1 => (n + 1, FracPrefix::Milli),
            0..=i64::MAX => (n, FracPrefix::Unit),
        }
    }

    fn power(&self) -> i64 {
        match self {
            FracPrefix::Yocto => -8,
            FracPrefix::Zepto => -7,
            FracPrefix::Atto => -6,
            FracPrefix::Femto => -5,
            FracPrefix::Pico => -4,
            FracPrefix::Nano => -3,
            FracPrefix::Micro => -2,
            FracPrefix::Milli => -1,
            FracPrefix::Unit => 0,
        }
    }

    fn prefix(&self) -> &'static str {
        match self {
            FracPrefix::Yocto => "y",
            FracPrefix::Zepto => "z",
            FracPrefix::Atto => "a",
            FracPrefix::Femto => "f",
            FracPrefix::Pico => "p",
            FracPrefix::Nano => "n",
            FracPrefix::Micro => "µ",
            FracPrefix::Milli => "m",
            FracPrefix::Unit => "",
        }
    }
}

impl Display for FracPrefix {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.prefix())
    }
}
