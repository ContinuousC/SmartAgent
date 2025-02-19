/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::prefix_trait::Prefix;
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
pub enum SiPrefix {
    Yocto,
    Zepto,
    Atto,
    Femto,
    Pico,
    Nano,
    Micro,
    Milli,
    Centi,
    Deci,
    Unit,
    Deca,
    Hecto,
    Kilo,
    Mega,
    Giga,
    Tera,
    Peta,
    Exa,
    Zetta,
    Yotta,
}

static SI_PREFIXES: [SiPrefix; 21] = [
    SiPrefix::Yocto,
    SiPrefix::Zepto,
    SiPrefix::Atto,
    SiPrefix::Femto,
    SiPrefix::Pico,
    SiPrefix::Nano,
    SiPrefix::Micro,
    SiPrefix::Milli,
    SiPrefix::Centi,
    SiPrefix::Deci,
    SiPrefix::Unit,
    SiPrefix::Deca,
    SiPrefix::Hecto,
    SiPrefix::Kilo,
    SiPrefix::Mega,
    SiPrefix::Giga,
    SiPrefix::Tera,
    SiPrefix::Peta,
    SiPrefix::Exa,
    SiPrefix::Zetta,
    SiPrefix::Yotta,
];

impl Prefix for SiPrefix {
    const BASE: u64 = 10;
    const SCALE: &[Self] = &SI_PREFIXES;

    fn from_power(n: i64) -> (i64, Self) {
        match n {
            i64::MIN..=-24 => (n + 24, SiPrefix::Yocto),
            -23..=-21 => (n + 21, SiPrefix::Zepto),
            -20..=-18 => (n + 18, SiPrefix::Atto),
            -17..=-15 => (n + 15, SiPrefix::Femto),
            -14..=-12 => (n + 12, SiPrefix::Pico),
            -11..=-9 => (n + 9, SiPrefix::Nano),
            -8..=-6 => (n + 6, SiPrefix::Micro),
            -5..=-3 => (n + 3, SiPrefix::Milli),
            -2 => (n + 2, SiPrefix::Centi),
            -1 => (n + 1, SiPrefix::Deci),
            0 => (n, SiPrefix::Unit),
            1 => (n - 1, SiPrefix::Deca),
            2 => (n - 2, SiPrefix::Hecto),
            3..=5 => (n - 3, SiPrefix::Kilo),
            6..=8 => (n - 6, SiPrefix::Mega),
            9..=11 => (n - 9, SiPrefix::Giga),
            12..=14 => (n - 12, SiPrefix::Tera),
            15..=17 => (n - 15, SiPrefix::Peta),
            18..=20 => (n - 18, SiPrefix::Exa),
            21..=23 => (n - 21, SiPrefix::Zetta),
            24..=i64::MAX => (n - 24, SiPrefix::Yotta),
        }
    }

    fn power(&self) -> i64 {
        match self {
            SiPrefix::Yocto => -24,
            SiPrefix::Zepto => -21,
            SiPrefix::Atto => -18,
            SiPrefix::Femto => -15,
            SiPrefix::Pico => -12,
            SiPrefix::Nano => -9,
            SiPrefix::Micro => -6,
            SiPrefix::Milli => -3,
            SiPrefix::Centi => -2,
            SiPrefix::Deci => -1,
            SiPrefix::Unit => 0,
            SiPrefix::Deca => 1,
            SiPrefix::Hecto => 2,
            SiPrefix::Kilo => 3,
            SiPrefix::Mega => 6,
            SiPrefix::Giga => 9,
            SiPrefix::Tera => 12,
            SiPrefix::Peta => 15,
            SiPrefix::Exa => 18,
            SiPrefix::Zetta => 21,
            SiPrefix::Yotta => 24,
        }
    }

    fn prefix(&self) -> &'static str {
        match self {
            SiPrefix::Yocto => "y",
            SiPrefix::Zepto => "z",
            SiPrefix::Atto => "a",
            SiPrefix::Femto => "f",
            SiPrefix::Pico => "p",
            SiPrefix::Nano => "n",
            SiPrefix::Micro => "µ",
            SiPrefix::Milli => "m",
            SiPrefix::Centi => "c",
            SiPrefix::Deci => "d",
            SiPrefix::Unit => "",
            SiPrefix::Deca => "da",
            SiPrefix::Hecto => "h",
            SiPrefix::Kilo => "k",
            SiPrefix::Mega => "M",
            SiPrefix::Giga => "G",
            SiPrefix::Tera => "T",
            SiPrefix::Peta => "P",
            SiPrefix::Exa => "E",
            SiPrefix::Zetta => "Z",
            SiPrefix::Yotta => "Y",
        }
    }
}

impl Display for SiPrefix {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.prefix())
    }
}
