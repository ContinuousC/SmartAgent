/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::convert::TryFrom;
use std::fmt::{self, Display, Formatter};
use std::ops::{Div, Mul};
use std::str::FromStr;

use crate::SiPrefix;

use super::{BaseUnit, Dimension, Quantity, UnitError};
//use super::power::{Square,Cubic,Inv,InvSquare,InvCubic};
use super::parser::{parse_composite_unit, parse_unit};
use super::{
    ConductivityUnit, CurrentUnit, DimensionlessUnit, FanSpeedUnit,
    FrequencyUnit, InformationUnit, LengthUnit, MassUnit, OperationUnit,
    PotentialUnit, PowerUnit, ResistanceUnit, TemperatureUnit, TimeUnit,
};

/// Supported unit and prefix combinations, grouped by dimension.
///
/// This is a static system with hand-coded conversions between
/// dimensions, which has the advantage of being light-weight
/// (the enum fits in a register). The drawback is that derived
/// dimensions must be defined before they can be used (even if
/// used only in intermediate values).
///
/// Only one unit per base dimension is supported (eg. no m*cm).
#[derive(PartialEq, Eq, Ord, PartialOrd, Hash, Clone, Copy, Debug)]
#[cfg_attr(
    not(feature = "serialize_as_string"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(
    feature = "serialize_as_string",
    derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr)
)]
#[cfg_attr(
    all(feature = "schemars", not(feature = "serialize_as_string")),
    derive(schemars::JsonSchema)
)]
#[cfg_attr(feature = "tsify", derive(tsify::Tsify))]
#[cfg_attr(
    all(feature = "tsify", feature = "serialize_as_string"),
    tsify(type = "string", from_wasm_abi, into_wasm_abi)
)]
pub enum Unit {
    Information(InformationUnit),
    Operations(OperationUnit),
    Length(LengthUnit),
    Mass(MassUnit),
    Time(TimeUnit),
    TimeSquare(TimeUnit),
    Temperature(TemperatureUnit),
    Current(CurrentUnit),
    Potential(PotentialUnit),
    Power(PowerUnit),
    Resistance(ResistanceUnit),
    Conductivity(ConductivityUnit),
    Area(LengthUnit),
    Volume(LengthUnit),
    Speed(LengthUnit, TimeUnit),
    Acceleration(LengthUnit, TimeUnit),
    AbsoluteHumidity(MassUnit, LengthUnit),
    Bandwidth(InformationUnit, TimeUnit),
    IOLatency(TimeUnit, OperationUnit),
    IOPerformance(OperationUnit, TimeUnit),
    AvgOpSize(InformationUnit, OperationUnit),
    Frequency(FrequencyUnit),
    FanSpeed(FanSpeedUnit),
    Dimensionless(DimensionlessUnit),
}

pub const NEUTRAL_UNIT: Unit =
    Unit::Dimensionless(DimensionlessUnit::REFERENCE);

impl Unit {
    pub fn parse(input: &str) -> Result<Self, UnitError> {
        parse_unit(input)
    }

    pub fn parse_composite(input: &str) -> Result<Self, UnitError> {
        parse_composite_unit(input)
    }

    pub fn dimension(&self) -> Dimension {
        match self {
            Unit::Information(_) => Dimension::Information,
            Unit::Operations(_) => Dimension::Operations,
            Unit::Length(_) => Dimension::Length,
            Unit::Mass(_) => Dimension::Mass,
            Unit::Time(_) => Dimension::Time,
            Unit::TimeSquare(_) => Dimension::TimeSquare,
            Unit::Temperature(_) => Dimension::Temperature,
            Unit::Current(_) => Dimension::Current,
            Unit::Potential(_) => Dimension::Potential,
            Unit::Power(_) => Dimension::Power,
            Unit::Resistance(_) => Dimension::Resistance,
            Unit::Conductivity(_) => Dimension::Conductivity,
            Unit::Area(_) => Dimension::Area,
            Unit::Volume(_) => Dimension::Volume,
            Unit::Speed(_, _) => Dimension::Speed,
            Unit::Acceleration(_, _) => Dimension::Acceleration,
            Unit::AbsoluteHumidity(_, _) => Dimension::AbsoluteHumidity,
            Unit::Frequency(_) => Dimension::Frequency,
            Unit::FanSpeed(_) => Dimension::FanSpeed,
            Unit::Bandwidth(_, _) => Dimension::Bandwidth,
            Unit::IOLatency(_, _) => Dimension::IOLatency,
            Unit::IOPerformance(_, _) => Dimension::IOPerformance,
            Unit::AvgOpSize(_, _) => Dimension::AvgOpSize,
            Unit::Dimensionless(_) => Dimension::Dimensionless,
        }
    }

    pub fn normalize(&self) -> Self {
        match self {
            Unit::Information(u) => Unit::Information(u.normalize()),
            Unit::Operations(u) => Unit::Operations(u.normalize()),
            Unit::Length(u) => Unit::Length(u.normalize()),
            Unit::Mass(u) => Unit::Mass(u.normalize()),
            Unit::Time(u) => Unit::Time(u.normalize()),
            Unit::Temperature(u) => Unit::Temperature(u.normalize()),
            Unit::Current(u) => Unit::Current(u.normalize()),
            Unit::Potential(u) => Unit::Potential(u.normalize()),
            Unit::Power(u) => Unit::Power(u.normalize()),
            Unit::Resistance(u) => Unit::Resistance(u.normalize()),
            Unit::Conductivity(u) => Unit::Conductivity(u.normalize()),
            Unit::Area(u) => Unit::Area(u.normalize()),
            Unit::Volume(u) => Unit::Volume(u.normalize()),
            Unit::Speed(l, t) => Unit::Speed(l.normalize(), t.normalize()),
            Unit::Acceleration(l, t) => {
                Unit::Acceleration(l.normalize(), t.normalize())
            }
            Unit::TimeSquare(u) => Unit::TimeSquare(u.normalize()),
            Unit::AbsoluteHumidity(_m, l) => Unit::AbsoluteHumidity(
                MassUnit::Gram(SiPrefix::Unit),
                l.normalize(),
            ),
            Unit::Frequency(u) => Unit::Frequency(u.normalize()),
            Unit::FanSpeed(u) => Unit::FanSpeed(u.normalize()),
            Unit::Bandwidth(i, t) => {
                Unit::Bandwidth(i.normalize(), t.normalize())
            }
            Unit::IOLatency(t, n) => {
                Unit::IOLatency(t.normalize(), n.normalize())
            }
            Unit::IOPerformance(n, t) => {
                Unit::IOPerformance(n.normalize(), t.normalize())
            }
            Unit::AvgOpSize(i, n) => {
                Unit::AvgOpSize(i.normalize(), n.normalize())
            }
            Unit::Dimensionless(u) => Unit::Dimensionless(u.normalize()),
        }
    }

    /// Returns a list of auto-scaling options.
    pub fn scale(&self) -> Vec<Self> {
        match self {
            Unit::Information(u) => {
                u.scale().into_iter().map(Unit::Information).collect()
            }
            Unit::Operations(u) => {
                u.scale().into_iter().map(Unit::Operations).collect()
            }
            Unit::Length(u) => {
                u.scale().into_iter().map(Unit::Length).collect()
            }
            Unit::Mass(u) => u.scale().into_iter().map(Unit::Mass).collect(),
            Unit::Time(u) => u.scale().into_iter().map(Unit::Time).collect(),
            Unit::TimeSquare(u) => {
                u.scale().into_iter().map(Unit::TimeSquare).collect()
            }
            Unit::Temperature(u) => {
                u.scale().into_iter().map(Unit::Temperature).collect()
            }
            Unit::Current(u) => {
                u.scale().into_iter().map(Unit::Current).collect()
            }
            Unit::Potential(u) => {
                u.scale().into_iter().map(Unit::Potential).collect()
            }
            Unit::Power(u) => u.scale().into_iter().map(Unit::Power).collect(),
            Unit::Resistance(u) => {
                u.scale().into_iter().map(Unit::Resistance).collect()
            }
            Unit::Conductivity(u) => {
                u.scale().into_iter().map(Unit::Conductivity).collect()
            }
            Unit::Area(u) => u.scale().into_iter().map(Unit::Area).collect(),
            Unit::Volume(u) => {
                u.scale().into_iter().map(Unit::Volume).collect()
            }
            Unit::Speed(u, v) => {
                u.scale().into_iter().map(|u| Unit::Speed(u, *v)).collect()
            }
            Unit::Acceleration(u, v) => u
                .scale()
                .into_iter()
                .map(|u| Unit::Acceleration(u, *v))
                .collect(),
            Unit::AbsoluteHumidity(u, v) => u
                .scale()
                .into_iter()
                .map(|u| Unit::AbsoluteHumidity(u, *v))
                .collect(),
            Unit::Bandwidth(u, v) => u
                .scale()
                .into_iter()
                .map(|u| Unit::Bandwidth(u, *v))
                .collect(),
            Unit::IOLatency(u, v) => u
                .scale()
                .into_iter()
                .map(|u| Unit::IOLatency(u, *v))
                .collect(),
            Unit::IOPerformance(u, v) => u
                .scale()
                .into_iter()
                .map(|u| Unit::IOPerformance(u, *v))
                .collect(),
            Unit::AvgOpSize(u, v) => u
                .scale()
                .into_iter()
                .map(|u| Unit::AvgOpSize(u, *v))
                .collect(),
            Unit::Frequency(u) => {
                u.scale().into_iter().map(Unit::Frequency).collect()
            }
            Unit::FanSpeed(u) => {
                u.scale().into_iter().map(Unit::FanSpeed).collect()
            }
            Unit::Dimensionless(u) => {
                u.scale().into_iter().map(Unit::Dimensionless).collect()
            }
        }
    }

    pub fn convert(&self, other: &Self, val: f64) -> Result<f64, UnitError> {
        match self.dimension() == other.dimension() {
            true => Ok(other.delinearize(
                self.linearize(val) * self.multiplier() / other.multiplier(),
            )),
            false => {
                Err(UnitError::Conversion(self.dimension(), other.dimension()))
            }
        }
    }

    fn multiplier(&self) -> f64 {
        match self {
            Unit::Information(u) => u.multiplier(),
            Unit::Operations(u) => u.multiplier(),
            Unit::Length(u) => u.multiplier(),
            Unit::Mass(u) => u.multiplier(),
            Unit::Time(u) => u.multiplier(),
            Unit::Temperature(u) => u.multiplier(),
            Unit::Current(u) => u.multiplier(),
            Unit::Potential(u) => u.multiplier(),
            Unit::Power(u) => u.multiplier(),
            Unit::Resistance(u) => u.multiplier(),
            Unit::Conductivity(u) => u.multiplier(),
            Unit::Area(u) => u.multiplier().powi(2),
            Unit::Volume(u) => u.multiplier().powi(3),
            Unit::Speed(l, t) => l.multiplier() / t.multiplier(),
            Unit::Acceleration(l, t) => l.multiplier() / t.multiplier().powi(2),
            Unit::TimeSquare(u) => u.multiplier().powi(2),
            Unit::AbsoluteHumidity(m, l) => {
                m.multiplier() / l.multiplier().powi(3)
            }
            Unit::Frequency(u) => u.multiplier(),
            Unit::FanSpeed(u) => u.multiplier(),
            Unit::Bandwidth(i, t) => i.multiplier() / t.multiplier(),
            Unit::IOLatency(t, n) => t.multiplier() / n.multiplier(),
            Unit::IOPerformance(n, t) => n.multiplier() / t.multiplier(),
            Unit::AvgOpSize(i, n) => i.multiplier() / n.multiplier(),
            Unit::Dimensionless(u) => u.multiplier(),
        }
    }

    pub(super) fn linearize(&self, n: f64) -> f64 {
        match self {
            Self::Power(u) => u.linearize(n),
            Self::Temperature(u) => n + u.offset(),
            _ => n,
        }
    }

    pub(super) fn delinearize(&self, n: f64) -> f64 {
        match self {
            Self::Power(u) => u.delinearize(n),
            Self::Temperature(u) => n - u.offset(),
            _ => n,
        }
    }

    pub fn quantity_from_json(
        &self,
        value: serde_json::Value,
    ) -> Result<Quantity, UnitError> {
        let val: f64 = serde_json::from_value(value)
            .map_err(|e| UnitError::Json(e.to_string()))?;
        Quantity(val, self.normalize()).convert(self)
    }
}

/* Display. */

impl Display for Unit {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        match self {
            Unit::Information(u) => u.fmt(f),
            Unit::Operations(u) => u.fmt(f),
            Unit::Length(u) => u.fmt(f),
            Unit::Mass(u) => u.fmt(f),
            Unit::Time(u) => u.fmt(f),
            Unit::Temperature(u) => u.fmt(f),
            Unit::Current(u) => u.fmt(f),
            Unit::Potential(u) => u.fmt(f),
            Unit::Power(u) => u.fmt(f),
            Unit::Resistance(u) => u.fmt(f),
            Unit::Conductivity(u) => u.fmt(f),
            Unit::Frequency(u) => u.fmt(f),
            Unit::FanSpeed(u) => u.fmt(f),
            Unit::Dimensionless(u) => u.fmt(f),
            Unit::Bandwidth(i, t) => write!(f, "{}/{}", i, t),
            Unit::IOLatency(t, n) => write!(f, "{}/{}", t, n),
            Unit::IOPerformance(n, t) => write!(f, "{}/{}", n, t),
            Unit::AvgOpSize(i, n) => write!(f, "{}/{}", i, n),
            Unit::Speed(l, t) => write!(f, "{}/{}", l, t),
            Unit::Acceleration(l, t) => write!(f, "{}/{}^2", l, t),
            Unit::TimeSquare(u) => write!(f, "{}^2", u),
            Unit::Area(u) => write!(f, "{}^2", u),
            Unit::Volume(u) => write!(f, "{}^3", u),
            Unit::AbsoluteHumidity(m, l) => write!(f, "{}/{}^3", m, l),
        }
    }
}

impl FromStr for Unit {
    type Err = UnitError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Unit::parse_composite(s)
    }
}

/* Operations on units. */

impl Unit {
    pub fn powi(self, n: i32) -> Result<Quantity, UnitError> {
        match (self, n) {
            (u, 1) => Ok(Quantity(1.0, u)),
            (_, 0) => Ok(Quantity(
                1.0,
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Dimensionless(u), n) => {
                Ok(Quantity(u.multiplier().powi(n - 1), Unit::Dimensionless(u)))
            }
            (Unit::Conductivity(c), -1) => Ok(Quantity(
                1.0 / c.multiplier(),
                Unit::Resistance(ResistanceUnit::REFERENCE),
            )),
            (Unit::Frequency(f), -2) => Ok(Quantity(
                1.0 / f.multiplier().powi(2),
                Unit::TimeSquare(TimeUnit::REFERENCE),
            )),
            (Unit::Frequency(f), -1) => Ok(Quantity(
                1.0 / f.multiplier(),
                Unit::Time(TimeUnit::REFERENCE),
            )),
            (Unit::IOLatency(t, n), -1) => {
                Ok(Quantity(1.0, Unit::IOPerformance(n, t)))
            }
            (Unit::IOPerformance(n, t), -1) => {
                Ok(Quantity(1.0, Unit::IOLatency(t, n)))
            }
            (Unit::Length(l), 2) => Ok(Quantity(1.0, Unit::Area(l))),
            (Unit::Length(l), 3) => Ok(Quantity(1.0, Unit::Volume(l))),
            (Unit::Resistance(r), -1) => Ok(Quantity(
                1.0 / r.multiplier(),
                Unit::Conductivity(ConductivityUnit::REFERENCE),
            )),
            (Unit::Time(t), -1) => Ok(Quantity(
                1.0 / t.multiplier(),
                Unit::Frequency(FrequencyUnit::REFERENCE),
            )),
            (Unit::Time(t), 2) => Ok(Quantity(1.0, Unit::TimeSquare(t))),
            _ => Err(UnitError::Pow(self.dimension(), n)),
        }
    }

    /* Whole unit operations for composite unit construction. */

    pub fn powi_unwrapped(self, n: i32) -> Result<Unit, UnitError> {
        match (self, n) {
            (u, 1) => Ok(u),
            (_, 0) => Ok(NEUTRAL_UNIT),
            (NEUTRAL_UNIT, _) => Ok(NEUTRAL_UNIT),
            (Unit::IOLatency(t, n), -1) => Ok(Unit::IOPerformance(n, t)),
            (Unit::IOPerformance(n, t), -1) => Ok(Unit::IOLatency(t, n)),
            (Unit::Length(l), 2) => Ok(Unit::Area(l)),
            (Unit::Length(l), 3) => Ok(Unit::Volume(l)),
            (Unit::Time(t), 2) => Ok(Unit::TimeSquare(t)),
            (Unit::Time(t), -1) => {
                Ok(Unit::Frequency(FrequencyUnit::PerTime(t)))
            }
            _ => Err(UnitError::CPow(self, n)),
        }
    }

    pub fn mul_unwrapped(self, rhs: Unit) -> Result<Unit, UnitError> {
        match (self, rhs) {
            (NEUTRAL_UNIT, u) => Ok(u),
            (u, NEUTRAL_UNIT) => Ok(u),
            (Unit::AbsoluteHumidity(ml, ll), Unit::Volume(lr)) => {
                match lr == ll {
                    true => Ok(Unit::Mass(ml)),
                    false => Err(UnitError::CMul(self, rhs)),
                }
            }
            (Unit::Acceleration(ll, tl), Unit::Time(tr)) => match tr == tl {
                true => Ok(Unit::Speed(ll, tr)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Acceleration(ll, tl), Unit::TimeSquare(tr)) => {
                match tr == tl {
                    true => Ok(Unit::Length(ll)),
                    false => Err(UnitError::CMul(self, rhs)),
                }
            }
            (Unit::Area(ll), Unit::Length(lr)) => match lr == ll {
                true => Ok(Unit::Volume(lr)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::AvgOpSize(il, nl), Unit::IOPerformance(nr, tr)) => {
                match nr == nl {
                    true => Ok(Unit::Bandwidth(il, tr)),
                    false => Err(UnitError::CMul(self, rhs)),
                }
            }
            (Unit::AvgOpSize(il, nl), Unit::Operations(nr)) => match nr == nl {
                true => Ok(Unit::Information(il)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Bandwidth(il, tl), Unit::IOLatency(tr, nr)) => {
                match tr == tl {
                    true => Ok(Unit::AvgOpSize(il, nr)),
                    false => Err(UnitError::CMul(self, rhs)),
                }
            }
            (Unit::Bandwidth(il, tl), Unit::Time(tr)) => match tr == tl {
                true => Ok(Unit::Information(il)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::IOLatency(tl, nl), Unit::Bandwidth(ir, tr)) => {
                match tr == tl {
                    true => Ok(Unit::AvgOpSize(ir, nl)),
                    false => Err(UnitError::CMul(self, rhs)),
                }
            }
            (Unit::IOLatency(tl, nl), Unit::Operations(nr)) => match nr == nl {
                true => Ok(Unit::Time(tl)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::IOPerformance(nl, tl), Unit::AvgOpSize(ir, nr)) => {
                match nr == nl {
                    true => Ok(Unit::Bandwidth(ir, tl)),
                    false => Err(UnitError::CMul(self, rhs)),
                }
            }
            (Unit::IOPerformance(nl, tl), Unit::Time(tr)) => match tr == tl {
                true => Ok(Unit::Operations(nl)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Length(ll), Unit::Area(lr)) => match lr == ll {
                true => Ok(Unit::Volume(lr)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Length(ll), Unit::Length(lr)) => match lr == ll {
                true => Ok(Unit::Area(lr)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Operations(nl), Unit::AvgOpSize(ir, nr)) => match nr == nl {
                true => Ok(Unit::Information(ir)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Operations(nl), Unit::IOLatency(tr, nr)) => match nr == nl {
                true => Ok(Unit::Time(tr)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Speed(ll, tl), Unit::Time(tr)) => match tr == tl {
                true => Ok(Unit::Length(ll)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Time(tl), Unit::Acceleration(lr, tr)) => match tr == tl {
                true => Ok(Unit::Speed(lr, tr)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Time(tl), Unit::Bandwidth(ir, tr)) => match tr == tl {
                true => Ok(Unit::Information(ir)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Time(tl), Unit::IOPerformance(nr, tr)) => match tr == tl {
                true => Ok(Unit::Operations(nr)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Time(tl), Unit::Speed(lr, tr)) => match tr == tl {
                true => Ok(Unit::Length(lr)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::Time(tl), Unit::Time(tr)) => match tr == tl {
                true => Ok(Unit::TimeSquare(tr)),
                false => Err(UnitError::CMul(self, rhs)),
            },
            (Unit::TimeSquare(tl), Unit::Acceleration(lr, tr)) => {
                match tr == tl {
                    true => Ok(Unit::Length(lr)),
                    false => Err(UnitError::CMul(self, rhs)),
                }
            }
            (Unit::Volume(ll), Unit::AbsoluteHumidity(mr, lr)) => {
                match lr == ll {
                    true => Ok(Unit::Mass(mr)),
                    false => Err(UnitError::CMul(self, rhs)),
                }
            }
            _ => Err(UnitError::CMul(self, rhs)),
        }
    }

    pub fn div_unwrapped(self, rhs: Unit) -> Result<Unit, UnitError> {
        match (self, rhs) {
            (u, NEUTRAL_UNIT) => Ok(u),
            (Unit::Area(ll), Unit::Length(lr)) => match lr == ll {
                true => Ok(Unit::Length(lr)),
                false => Err(UnitError::CDiv(self, rhs)),
            },
            (Unit::AvgOpSize(il, nl), Unit::Bandwidth(ir, tr)) => {
                match ir == il {
                    true => Ok(Unit::IOLatency(tr, nl)),
                    false => Err(UnitError::CDiv(self, rhs)),
                }
            }
            (Unit::AvgOpSize(il, nl), Unit::IOLatency(tr, nr)) => {
                match nr == nl {
                    true => Ok(Unit::Bandwidth(il, tr)),
                    false => Err(UnitError::CDiv(self, rhs)),
                }
            }
            (Unit::Bandwidth(il, tl), Unit::AvgOpSize(ir, nr)) => {
                match ir == il {
                    true => Ok(Unit::IOPerformance(nr, tl)),
                    false => Err(UnitError::CDiv(self, rhs)),
                }
            }
            (Unit::Bandwidth(il, tl), Unit::IOPerformance(nr, tr)) => {
                match tr == tl {
                    true => Ok(Unit::AvgOpSize(il, nr)),
                    false => Err(UnitError::CDiv(self, rhs)),
                }
            }
            (Unit::Information(il), Unit::AvgOpSize(ir, nr)) => {
                match ir == il {
                    true => Ok(Unit::Operations(nr)),
                    false => Err(UnitError::CDiv(self, rhs)),
                }
            }
            (Unit::Information(il), Unit::Bandwidth(ir, tr)) => {
                match ir == il {
                    true => Ok(Unit::Time(tr)),
                    false => Err(UnitError::CDiv(self, rhs)),
                }
            }
            (Unit::Information(il), Unit::Operations(nr)) => {
                Ok(Unit::AvgOpSize(il, nr))
            }
            (Unit::Information(il), Unit::Time(tr)) => {
                Ok(Unit::Bandwidth(il, tr))
            }
            (Unit::Length(ll), Unit::Acceleration(lr, tr)) => match lr == ll {
                true => Ok(Unit::TimeSquare(tr)),
                false => Err(UnitError::CDiv(self, rhs)),
            },
            (Unit::Length(ll), Unit::Speed(lr, tr)) => match lr == ll {
                true => Ok(Unit::Time(tr)),
                false => Err(UnitError::CDiv(self, rhs)),
            },
            (Unit::Length(ll), Unit::Time(tr)) => Ok(Unit::Speed(ll, tr)),
            (Unit::Length(ll), Unit::TimeSquare(tr)) => {
                Ok(Unit::Acceleration(ll, tr))
            }
            (Unit::Mass(ml), Unit::AbsoluteHumidity(mr, lr)) => {
                match mr == ml {
                    true => Ok(Unit::Volume(lr)),
                    false => Err(UnitError::CDiv(self, rhs)),
                }
            }
            (Unit::Mass(ml), Unit::Volume(lr)) => {
                Ok(Unit::AbsoluteHumidity(ml, lr))
            }
            (Unit::Operations(nl), Unit::IOPerformance(nr, tr)) => {
                match nr == nl {
                    true => Ok(Unit::Time(tr)),
                    false => Err(UnitError::CDiv(self, rhs)),
                }
            }
            (Unit::Operations(nl), Unit::Time(tr)) => {
                Ok(Unit::IOPerformance(nl, tr))
            }
            (Unit::Speed(ll, tl), Unit::Acceleration(lr, tr)) => {
                match tr == tl && lr == ll {
                    true => Ok(Unit::Time(tr)),
                    false => Err(UnitError::CDiv(self, rhs)),
                }
            }
            (Unit::Speed(ll, tl), Unit::Time(tr)) => match tr == tl {
                true => Ok(Unit::Acceleration(ll, tr)),
                false => Err(UnitError::CDiv(self, rhs)),
            },
            (Unit::Time(tl), Unit::IOLatency(tr, nr)) => match tr == tl {
                true => Ok(Unit::Operations(nr)),
                false => Err(UnitError::CDiv(self, rhs)),
            },
            (Unit::Time(tl), Unit::Operations(nr)) => {
                Ok(Unit::IOLatency(tl, nr))
            }
            (Unit::TimeSquare(tl), Unit::Time(tr)) => match tr == tl {
                true => Ok(Unit::Time(tr)),
                false => Err(UnitError::CDiv(self, rhs)),
            },
            (Unit::Volume(ll), Unit::Area(lr)) => match lr == ll {
                true => Ok(Unit::Length(lr)),
                false => Err(UnitError::CDiv(self, rhs)),
            },
            (Unit::Volume(ll), Unit::Length(lr)) => match lr == ll {
                true => Ok(Unit::Area(lr)),
                false => Err(UnitError::CDiv(self, rhs)),
            },
            _ => Err(UnitError::CDiv(self, rhs)),
        }
    }
}

/* Multiply / divide units. The result is a quantity specifying a factor and a unit. This is used to find a
 * suitable output unit for these operations. Note, however, that this does *not* take into account offsets
 * and linearization / delinearization. */

impl Mul<Unit> for Unit {
    type Output = Result<Quantity, UnitError>;
    fn mul(self, rhs: Unit) -> Self::Output {
        match (self, rhs) {
            (Unit::Dimensionless(d), u) => Ok(Quantity(d.multiplier(), u)),
            (u, Unit::Dimensionless(d)) => Ok(Quantity(d.multiplier(), u)),
            (Unit::AbsoluteHumidity(ml, ll), Unit::Volume(lr)) => Ok(Quantity(
                lr.multiplier().powi(3) / ll.multiplier().powi(3),
                Unit::Mass(ml),
            )),
            (Unit::Acceleration(ll, tl), Unit::Time(tr)) => Ok(Quantity(
                tr.multiplier() / tl.multiplier(),
                Unit::Speed(ll, tl),
            )),
            (Unit::Acceleration(ll, tl), Unit::TimeSquare(tr)) => Ok(Quantity(
                tr.multiplier().powi(2) / tl.multiplier().powi(2),
                Unit::Length(ll),
            )),
            (Unit::Area(ll), Unit::Length(lr)) => Ok(Quantity(
                lr.multiplier() / ll.multiplier(),
                Unit::Volume(ll),
            )),
            (Unit::AvgOpSize(il, nl), Unit::IOPerformance(nr, tr)) => {
                Ok(Quantity(
                    nr.multiplier() / nl.multiplier(),
                    Unit::Bandwidth(il, tr),
                ))
            }
            (Unit::AvgOpSize(il, nl), Unit::Operations(nr)) => Ok(Quantity(
                nr.multiplier() / nl.multiplier(),
                Unit::Information(il),
            )),
            (Unit::Bandwidth(il, tl), Unit::IOLatency(tr, nr)) => Ok(Quantity(
                tr.multiplier() / tl.multiplier(),
                Unit::AvgOpSize(il, nr),
            )),
            (Unit::Bandwidth(il, tl), Unit::Time(tr)) => Ok(Quantity(
                tr.multiplier() / tl.multiplier(),
                Unit::Information(il),
            )),
            (Unit::Conductivity(cl), Unit::Potential(vr)) => Ok(Quantity(
                cl.multiplier() * vr.multiplier(),
                Unit::Current(CurrentUnit::REFERENCE),
            )),
            (Unit::Conductivity(cl), Unit::Resistance(rr)) => Ok(Quantity(
                cl.multiplier() * rr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Current(al), Unit::Potential(vr)) => Ok(Quantity(
                al.multiplier() * vr.multiplier(),
                Unit::Power(PowerUnit::REFERENCE),
            )),
            (Unit::Current(al), Unit::Resistance(rr)) => Ok(Quantity(
                al.multiplier() * rr.multiplier(),
                Unit::Potential(PotentialUnit::REFERENCE),
            )),
            (Unit::Frequency(fl), Unit::Information(ir)) => Ok(Quantity(
                fl.multiplier(),
                Unit::Bandwidth(ir, TimeUnit::REFERENCE),
            )),
            (Unit::Frequency(fl), Unit::Length(lr)) => Ok(Quantity(
                fl.multiplier(),
                Unit::Speed(lr, TimeUnit::REFERENCE),
            )),
            (Unit::Frequency(fl), Unit::Operations(nr)) => Ok(Quantity(
                fl.multiplier(),
                Unit::IOPerformance(nr, TimeUnit::REFERENCE),
            )),
            (Unit::Frequency(fl), Unit::Speed(lr, tr)) => Ok(Quantity(
                tr.multiplier() * fl.multiplier(),
                Unit::Acceleration(lr, tr),
            )),
            (Unit::Frequency(fl), Unit::Time(tr)) => Ok(Quantity(
                fl.multiplier() * tr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Frequency(fl), Unit::TimeSquare(tr)) => {
                Ok(Quantity(tr.multiplier() * fl.multiplier(), Unit::Time(tr)))
            }
            (Unit::IOLatency(tl, nl), Unit::Bandwidth(ir, tr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier(),
                Unit::AvgOpSize(ir, nl),
            )),
            (Unit::IOLatency(tl, nl), Unit::IOPerformance(nr, tr)) => {
                Ok(Quantity(
                    nr.multiplier() * tl.multiplier()
                        / (nl.multiplier() * tr.multiplier()),
                    Unit::Dimensionless(DimensionlessUnit::REFERENCE),
                ))
            }
            (Unit::IOLatency(tl, nl), Unit::Operations(nr)) => {
                Ok(Quantity(nr.multiplier() / nl.multiplier(), Unit::Time(tl)))
            }
            (Unit::IOPerformance(nl, tl), Unit::AvgOpSize(ir, nr)) => {
                Ok(Quantity(
                    nl.multiplier() / nr.multiplier(),
                    Unit::Bandwidth(ir, tl),
                ))
            }
            (Unit::IOPerformance(nl, tl), Unit::IOLatency(tr, nr)) => {
                Ok(Quantity(
                    nl.multiplier() * tr.multiplier()
                        / (nr.multiplier() * tl.multiplier()),
                    Unit::Dimensionless(DimensionlessUnit::REFERENCE),
                ))
            }
            (Unit::IOPerformance(nl, tl), Unit::Time(tr)) => Ok(Quantity(
                tr.multiplier() / tl.multiplier(),
                Unit::Operations(nl),
            )),
            (Unit::Information(il), Unit::Frequency(fr)) => Ok(Quantity(
                fr.multiplier(),
                Unit::Bandwidth(il, TimeUnit::REFERENCE),
            )),
            (Unit::Length(ll), Unit::Area(lr)) => Ok(Quantity(
                ll.multiplier() / lr.multiplier(),
                Unit::Volume(lr),
            )),
            (Unit::Length(ll), Unit::Frequency(fr)) => Ok(Quantity(
                fr.multiplier(),
                Unit::Speed(ll, TimeUnit::REFERENCE),
            )),
            (Unit::Length(ll), Unit::Length(lr)) => {
                Ok(Quantity(ll.multiplier() / lr.multiplier(), Unit::Area(lr)))
            }
            (Unit::Operations(nl), Unit::AvgOpSize(ir, nr)) => Ok(Quantity(
                nl.multiplier() / nr.multiplier(),
                Unit::Information(ir),
            )),
            (Unit::Operations(nl), Unit::Frequency(fr)) => Ok(Quantity(
                fr.multiplier(),
                Unit::IOPerformance(nl, TimeUnit::REFERENCE),
            )),
            (Unit::Operations(nl), Unit::IOLatency(tr, nr)) => {
                Ok(Quantity(nl.multiplier() / nr.multiplier(), Unit::Time(tr)))
            }
            (Unit::Potential(vl), Unit::Conductivity(cr)) => Ok(Quantity(
                cr.multiplier() * vl.multiplier(),
                Unit::Current(CurrentUnit::REFERENCE),
            )),
            (Unit::Potential(vl), Unit::Current(ar)) => Ok(Quantity(
                ar.multiplier() * vl.multiplier(),
                Unit::Power(PowerUnit::REFERENCE),
            )),
            (Unit::Resistance(rl), Unit::Conductivity(cr)) => Ok(Quantity(
                cr.multiplier() * rl.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Resistance(rl), Unit::Current(ar)) => Ok(Quantity(
                ar.multiplier() * rl.multiplier(),
                Unit::Potential(PotentialUnit::REFERENCE),
            )),
            (Unit::Speed(ll, tl), Unit::Frequency(fr)) => Ok(Quantity(
                tl.multiplier() * fr.multiplier(),
                Unit::Acceleration(ll, tl),
            )),
            (Unit::Speed(ll, tl), Unit::Time(tr)) => Ok(Quantity(
                tr.multiplier() / tl.multiplier(),
                Unit::Length(ll),
            )),
            (Unit::Time(tl), Unit::Acceleration(lr, tr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier(),
                Unit::Speed(lr, tr),
            )),
            (Unit::Time(tl), Unit::Bandwidth(ir, tr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier(),
                Unit::Information(ir),
            )),
            (Unit::Time(tl), Unit::Frequency(fr)) => Ok(Quantity(
                fr.multiplier() * tl.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Time(tl), Unit::IOPerformance(nr, tr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier(),
                Unit::Operations(nr),
            )),
            (Unit::Time(tl), Unit::Speed(lr, tr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier(),
                Unit::Length(lr),
            )),
            (Unit::Time(tl), Unit::Time(tr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier(),
                Unit::TimeSquare(tr),
            )),
            (Unit::TimeSquare(tl), Unit::Acceleration(lr, tr)) => Ok(Quantity(
                tl.multiplier().powi(2) / tr.multiplier().powi(2),
                Unit::Length(lr),
            )),
            (Unit::TimeSquare(tl), Unit::Frequency(fr)) => {
                Ok(Quantity(tl.multiplier() * fr.multiplier(), Unit::Time(tl)))
            }
            (Unit::Volume(ll), Unit::AbsoluteHumidity(mr, lr)) => Ok(Quantity(
                ll.multiplier().powi(3) / lr.multiplier().powi(3),
                Unit::Mass(mr),
            )),
            _ => Err(UnitError::Mul(self.dimension(), rhs.dimension())),
        }
    }
}

impl Div<Unit> for Unit {
    type Output = Result<Quantity, UnitError>;
    fn div(self, rhs: Unit) -> Self::Output {
        match (self, rhs) {
            (u, Unit::Dimensionless(d)) => {
                Ok(Quantity(1.0 / d.multiplier(), u))
            }
            (
                Unit::AbsoluteHumidity(ml, ll),
                Unit::AbsoluteHumidity(mr, lr),
            ) => Ok(Quantity(
                lr.multiplier().powi(3) * ml.multiplier()
                    / (ll.multiplier().powi(3) * mr.multiplier()),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Acceleration(ll, tl), Unit::Acceleration(lr, tr)) => {
                Ok(Quantity(
                    ll.multiplier() * tr.multiplier().powi(2)
                        / (lr.multiplier() * tl.multiplier().powi(2)),
                    Unit::Dimensionless(DimensionlessUnit::REFERENCE),
                ))
            }
            (Unit::Acceleration(ll, tl), Unit::Frequency(fr)) => Ok(Quantity(
                1.0 / (tl.multiplier() * fr.multiplier()),
                Unit::Speed(ll, tl),
            )),
            (Unit::Acceleration(ll, tl), Unit::Speed(lr, tr)) => Ok(Quantity(
                ll.multiplier() * tr.multiplier()
                    / (lr.multiplier() * tl.multiplier().powi(2)),
                Unit::Frequency(FrequencyUnit::REFERENCE),
            )),
            (Unit::Area(ll), Unit::Area(lr)) => Ok(Quantity(
                ll.multiplier().powi(2) / lr.multiplier().powi(2),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Area(ll), Unit::Length(lr)) => Ok(Quantity(
                ll.multiplier() / lr.multiplier(),
                Unit::Length(ll),
            )),
            (Unit::AvgOpSize(il, nl), Unit::AvgOpSize(ir, nr)) => Ok(Quantity(
                il.multiplier() * nr.multiplier()
                    / (ir.multiplier() * nl.multiplier()),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::AvgOpSize(il, nl), Unit::Bandwidth(ir, tr)) => Ok(Quantity(
                il.multiplier() / ir.multiplier(),
                Unit::IOLatency(tr, nl),
            )),
            (Unit::AvgOpSize(il, nl), Unit::IOLatency(tr, nr)) => Ok(Quantity(
                nr.multiplier() / nl.multiplier(),
                Unit::Bandwidth(il, tr),
            )),
            (Unit::Bandwidth(il, tl), Unit::AvgOpSize(ir, nr)) => Ok(Quantity(
                il.multiplier() / ir.multiplier(),
                Unit::IOPerformance(nr, tl),
            )),
            (Unit::Bandwidth(il, tl), Unit::Bandwidth(ir, tr)) => Ok(Quantity(
                il.multiplier() * tr.multiplier()
                    / (ir.multiplier() * tl.multiplier()),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Bandwidth(il, tl), Unit::Frequency(fr)) => Ok(Quantity(
                1.0 / (fr.multiplier() * tl.multiplier()),
                Unit::Information(il),
            )),
            (Unit::Bandwidth(il, tl), Unit::IOPerformance(nr, tr)) => {
                Ok(Quantity(
                    tr.multiplier() / tl.multiplier(),
                    Unit::AvgOpSize(il, nr),
                ))
            }
            (Unit::Bandwidth(il, tl), Unit::Information(ir)) => Ok(Quantity(
                il.multiplier() / (ir.multiplier() * tl.multiplier()),
                Unit::Frequency(FrequencyUnit::REFERENCE),
            )),
            (Unit::Conductivity(cl), Unit::Conductivity(cr)) => Ok(Quantity(
                cl.multiplier() / cr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Current(al), Unit::Conductivity(cr)) => Ok(Quantity(
                al.multiplier() / cr.multiplier(),
                Unit::Potential(PotentialUnit::REFERENCE),
            )),
            (Unit::Current(al), Unit::Current(ar)) => Ok(Quantity(
                al.multiplier() / ar.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Current(al), Unit::Potential(vr)) => Ok(Quantity(
                al.multiplier() / vr.multiplier(),
                Unit::Conductivity(ConductivityUnit::REFERENCE),
            )),
            (Unit::FanSpeed(fl), Unit::FanSpeed(fr)) => Ok(Quantity(
                fl.multiplier() / fr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Frequency(fl), Unit::Frequency(fr)) => Ok(Quantity(
                fl.multiplier() / fr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::IOLatency(tl, nl), Unit::IOLatency(tr, nr)) => Ok(Quantity(
                nr.multiplier() * tl.multiplier()
                    / (nl.multiplier() * tr.multiplier()),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::IOPerformance(nl, tl), Unit::Frequency(fr)) => Ok(Quantity(
                1.0 / (fr.multiplier() * tl.multiplier()),
                Unit::Operations(nl),
            )),
            (Unit::IOPerformance(nl, tl), Unit::IOPerformance(nr, tr)) => {
                Ok(Quantity(
                    nl.multiplier() * tr.multiplier()
                        / (nr.multiplier() * tl.multiplier()),
                    Unit::Dimensionless(DimensionlessUnit::REFERENCE),
                ))
            }
            (Unit::IOPerformance(nl, tl), Unit::Operations(nr)) => {
                Ok(Quantity(
                    nl.multiplier() / (nr.multiplier() * tl.multiplier()),
                    Unit::Frequency(FrequencyUnit::REFERENCE),
                ))
            }
            (Unit::Information(il), Unit::AvgOpSize(ir, nr)) => Ok(Quantity(
                il.multiplier() / ir.multiplier(),
                Unit::Operations(nr),
            )),
            (Unit::Information(il), Unit::Bandwidth(ir, tr)) => {
                Ok(Quantity(il.multiplier() / ir.multiplier(), Unit::Time(tr)))
            }
            (Unit::Information(il), Unit::Information(ir)) => Ok(Quantity(
                il.multiplier() / ir.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Information(il), Unit::Operations(nr)) => {
                Ok(Quantity(1.0, Unit::AvgOpSize(il, nr)))
            }
            (Unit::Information(il), Unit::Time(tr)) => {
                Ok(Quantity(1.0, Unit::Bandwidth(il, tr)))
            }
            (Unit::Length(ll), Unit::Acceleration(lr, tr)) => Ok(Quantity(
                ll.multiplier() / lr.multiplier(),
                Unit::TimeSquare(tr),
            )),
            (Unit::Length(ll), Unit::Length(lr)) => Ok(Quantity(
                ll.multiplier() / lr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Length(ll), Unit::Speed(lr, tr)) => {
                Ok(Quantity(ll.multiplier() / lr.multiplier(), Unit::Time(tr)))
            }
            (Unit::Length(ll), Unit::Time(tr)) => {
                Ok(Quantity(1.0, Unit::Speed(ll, tr)))
            }
            (Unit::Length(ll), Unit::TimeSquare(tr)) => {
                Ok(Quantity(1.0, Unit::Acceleration(ll, tr)))
            }
            (Unit::Mass(ml), Unit::AbsoluteHumidity(mr, lr)) => Ok(Quantity(
                ml.multiplier() / mr.multiplier(),
                Unit::Volume(lr),
            )),
            (Unit::Mass(ml), Unit::Mass(mr)) => Ok(Quantity(
                ml.multiplier() / mr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Mass(ml), Unit::Volume(lr)) => {
                Ok(Quantity(1.0, Unit::AbsoluteHumidity(ml, lr)))
            }
            (Unit::Operations(nl), Unit::IOPerformance(nr, tr)) => {
                Ok(Quantity(nl.multiplier() / nr.multiplier(), Unit::Time(tr)))
            }
            (Unit::Operations(nl), Unit::Operations(nr)) => Ok(Quantity(
                nl.multiplier() / nr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Operations(nl), Unit::Time(tr)) => {
                Ok(Quantity(1.0, Unit::IOPerformance(nl, tr)))
            }
            (Unit::Potential(vl), Unit::Current(ar)) => Ok(Quantity(
                vl.multiplier() / ar.multiplier(),
                Unit::Resistance(ResistanceUnit::REFERENCE),
            )),
            (Unit::Potential(vl), Unit::Potential(vr)) => Ok(Quantity(
                vl.multiplier() / vr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Potential(vl), Unit::Resistance(rr)) => Ok(Quantity(
                vl.multiplier() / rr.multiplier(),
                Unit::Current(CurrentUnit::REFERENCE),
            )),
            (Unit::Power(wl), Unit::Current(ar)) => Ok(Quantity(
                wl.multiplier() / ar.multiplier(),
                Unit::Potential(PotentialUnit::REFERENCE),
            )),
            (Unit::Power(wl), Unit::Potential(vr)) => Ok(Quantity(
                wl.multiplier() / vr.multiplier(),
                Unit::Current(CurrentUnit::REFERENCE),
            )),
            (Unit::Power(wl), Unit::Power(wr)) => Ok(Quantity(
                wl.multiplier() / wr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Resistance(rl), Unit::Resistance(rr)) => Ok(Quantity(
                rl.multiplier() / rr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Speed(ll, tl), Unit::Acceleration(lr, tr)) => Ok(Quantity(
                tr.multiplier() * ll.multiplier()
                    / (tl.multiplier() * lr.multiplier()),
                Unit::Time(tr),
            )),
            (Unit::Speed(ll, tl), Unit::Frequency(fr)) => Ok(Quantity(
                1.0 / (fr.multiplier() * tl.multiplier()),
                Unit::Length(ll),
            )),
            (Unit::Speed(ll, tl), Unit::Length(lr)) => Ok(Quantity(
                ll.multiplier() / (lr.multiplier() * tl.multiplier()),
                Unit::Frequency(FrequencyUnit::REFERENCE),
            )),
            (Unit::Speed(ll, tl), Unit::Speed(lr, tr)) => Ok(Quantity(
                ll.multiplier() * tr.multiplier()
                    / (lr.multiplier() * tl.multiplier()),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Speed(ll, tl), Unit::Time(tr)) => Ok(Quantity(
                tr.multiplier() / tl.multiplier(),
                Unit::Acceleration(ll, tr),
            )),
            (Unit::Temperature(tl), Unit::Temperature(tr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Time(tl), Unit::Frequency(fr)) => Ok(Quantity(
                1.0 / (tl.multiplier() * fr.multiplier()),
                Unit::TimeSquare(tl),
            )),
            (Unit::Time(tl), Unit::IOLatency(tr, nr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier(),
                Unit::Operations(nr),
            )),
            (Unit::Time(tl), Unit::Operations(nr)) => {
                Ok(Quantity(1.0, Unit::IOLatency(tl, nr)))
            }
            (Unit::Time(tl), Unit::Time(tr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier(),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Time(tl), Unit::TimeSquare(tr)) => Ok(Quantity(
                tl.multiplier() / tr.multiplier().powi(2),
                Unit::Frequency(FrequencyUnit::REFERENCE),
            )),
            (Unit::TimeSquare(tl), Unit::Time(tr)) => {
                Ok(Quantity(tl.multiplier() / tr.multiplier(), Unit::Time(tl)))
            }
            (Unit::TimeSquare(tl), Unit::TimeSquare(tr)) => Ok(Quantity(
                tl.multiplier().powi(2) / tr.multiplier().powi(2),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            (Unit::Volume(ll), Unit::Area(lr)) => Ok(Quantity(
                ll.multiplier().powi(2) / lr.multiplier().powi(2),
                Unit::Length(ll),
            )),
            (Unit::Volume(ll), Unit::Length(lr)) => {
                Ok(Quantity(ll.multiplier() / lr.multiplier(), Unit::Area(ll)))
            }
            (Unit::Volume(ll), Unit::Volume(lr)) => Ok(Quantity(
                ll.multiplier().powi(3) / lr.multiplier().powi(3),
                Unit::Dimensionless(DimensionlessUnit::REFERENCE),
            )),
            _ => Err(UnitError::Div(self.dimension(), rhs.dimension())),
        }
    }
}

impl From<Unit> for String {
    fn from(val: Unit) -> Self {
        format!("{}", val)
    }
}

impl TryFrom<String> for Unit {
    type Error = UnitError;
    fn try_from(val: String) -> std::result::Result<Self, Self::Error> {
        Self::parse(&val)
    }
}

#[cfg(all(feature = "schemars", feature = "serialize_as_string"))]
impl schemars::JsonSchema for Unit {
    fn schema_name() -> String {
        String::from("Unit")
    }

    fn json_schema(
        gen: &mut schemars::gen::SchemaGenerator,
    ) -> schemars::schema::Schema {
        String::json_schema(gen)
    }
}
