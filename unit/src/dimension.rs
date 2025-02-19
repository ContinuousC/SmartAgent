/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::base_unit::{
    BaseUnit, ConductivityUnit, CurrentUnit, DimensionlessUnit, FanSpeedUnit,
    FrequencyUnit, InformationUnit, LengthUnit, MassUnit, OperationUnit,
    PotentialUnit, PowerUnit, ResistanceUnit, TemperatureUnit, TimeUnit,
};
use super::error::UnitError;
use super::prefix::DecPrefix;
use super::unit::Unit;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};
use std::mem::MaybeUninit;
use std::ops::{Add, Div, Mul, Sub};

/// Base dimensions. These can be considered a unit's "type".
/// Conversion is possible only between units of the same
/// dimension.

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
#[cfg_attr(feature = "serialize_as_string", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "tsify", tsify(from_wasm_abi, into_wasm_abi))]
pub enum Dimension {
    /* SI base dimensions. */
    Length,
    Mass,
    Time,
    Current,
    Temperature,
    /* AmountOfSubstance */
    /* LuminousIntensity */

    /* SI derived dimensions. */
    Area,
    Volume,
    Speed,
    Acceleration,
    Potential,
    Power,
    Resistance,
    Conductivity,
    AbsoluteHumidity,
    Frequency,

    /* Intermediate. */
    TimeSquare,

    /* Information. */
    Information,
    Operations,
    Bandwidth,
    IOLatency,
    IOPerformance,
    AvgOpSize,

    /* Special. */
    FanSpeed, /* we want this saved in rpm, not Hz. */

    /* Dimensionless */
    Dimensionless,
}

pub(crate) static DIMENSIONS: [Dimension; 24] = [
    Dimension::Length,
    Dimension::Mass,
    Dimension::Time,
    Dimension::Current,
    Dimension::Potential,
    Dimension::Power,
    Dimension::Resistance,
    Dimension::Conductivity,
    Dimension::Temperature,
    Dimension::Area,
    Dimension::Volume,
    Dimension::Speed,
    Dimension::Acceleration,
    Dimension::TimeSquare,
    Dimension::AbsoluteHumidity,
    Dimension::Frequency,
    Dimension::FanSpeed,
    Dimension::Information,
    Dimension::Operations,
    Dimension::Bandwidth,
    Dimension::IOLatency,
    Dimension::IOPerformance,
    Dimension::AvgOpSize,
    Dimension::Dimensionless,
];

macro_rules! define_units {
    ($name:ident, $unit:ident, $units:ident) => {
        pub(crate) static $name: [Unit; $units::LIST.len()] = {
            let mut r = [MaybeUninit::uninit(); $units::LIST.len()];
            let mut i = 0;
            while i < $units::LIST.len() {
                r[i] = MaybeUninit::new(Unit::$unit($units::LIST[i]));
                i += 1;
            }
            unsafe { std::mem::transmute(r) }
        };
    };
    ($name:ident, $unit:ident, $unitsA:ident, $unitsB:ident) => {
        pub(crate) static $name: [Unit;
            $unitsA::LIST.len() * $unitsB::LIST.len()] = {
            let mut r = [MaybeUninit::uninit();
                $unitsA::LIST.len() * $unitsB::LIST.len()];
            let mut i = 0;
            while i < $unitsA::LIST.len() {
                let mut j = 0;
                while j < $unitsB::LIST.len() {
                    r[i * $unitsB::LIST.len() + j] = MaybeUninit::new(
                        Unit::$unit($unitsA::LIST[i], $unitsB::LIST[j]),
                    );
                    j += 1;
                }
                i += 1;
            }
            unsafe { std::mem::transmute(r) }
        };
    };
}

define_units!(LENGTH_UNITS, Length, LengthUnit);
define_units!(MASS_UNITS, Mass, MassUnit);
define_units!(TIME_UNITS, Time, TimeUnit);
define_units!(CURRENT_UNITS, Current, CurrentUnit);
define_units!(TEMPERATURE_UNITS, Temperature, TemperatureUnit);
define_units!(AREA_UNITS, Area, LengthUnit);
define_units!(VOLUME_UNITS, Volume, LengthUnit);
define_units!(SPEED_UNITS, Speed, LengthUnit, TimeUnit);
define_units!(ACCELERATION_UNITS, Acceleration, LengthUnit, TimeUnit);
define_units!(POTENTIAL_UNITS, Potential, PotentialUnit);
define_units!(POWER_UNITS, Power, PowerUnit);
define_units!(RESISTANCE_UNITS, Resistance, ResistanceUnit);
define_units!(CONDUCTIVITY_UNITS, Conductivity, ConductivityUnit);
define_units!(
    ABSOLUTE_HUMIDITY_UNITS,
    AbsoluteHumidity,
    MassUnit,
    LengthUnit
);
define_units!(FREQUENCY_UNITS, Frequency, FrequencyUnit);
define_units!(TIME_SQUARE_UNITS, TimeSquare, TimeUnit);
define_units!(INFORMATION_UNITS, Information, InformationUnit);
define_units!(OPERATIONS_UNITS, Operations, OperationUnit);
define_units!(BANDWIDTH_UNITS, Bandwidth, InformationUnit, TimeUnit);
define_units!(IO_LATENCY_UNITS, IOLatency, TimeUnit, OperationUnit);
define_units!(IO_PERFORMANCE_UNITS, IOPerformance, OperationUnit, TimeUnit);
define_units!(AVG_OP_SIZE_UNITS, AvgOpSize, InformationUnit, OperationUnit);
define_units!(FAN_SPEED_UNITS, FanSpeed, FanSpeedUnit);
define_units!(DIMENSIONLESS_UNITS, Dimensionless, DimensionlessUnit);

impl Dimension {
    pub const LIST: &[Self] = &DIMENSIONS;

    pub const fn symbol(&self) -> &str {
        match self {
            Dimension::Length => "L",
            Dimension::Mass => "M",
            Dimension::Time => "T",
            Dimension::Current => "I",
            Dimension::Potential => "U",
            Dimension::Power => "P",
            Dimension::Resistance => "R",
            Dimension::Conductivity => "C",
            Dimension::Temperature => "Θ",
            Dimension::Area => "A",
            Dimension::Volume => "V",
            Dimension::Speed => "v",
            Dimension::Acceleration => "a",
            Dimension::TimeSquare => "T^2",
            Dimension::AbsoluteHumidity => "",
            Dimension::Frequency => "f",
            Dimension::FanSpeed => "f",
            Dimension::Information => "i",  // ?
            Dimension::Operations => "op",  // ?
            Dimension::Bandwidth => "i/T",  // ?
            Dimension::IOLatency => "T/op", // ?
            Dimension::IOPerformance => "op/T", // ?
            Dimension::AvgOpSize => "i/op",
            Dimension::Dimensionless => "",
        }
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Dimension::Length => "length",
            Dimension::Mass => "mass",
            Dimension::Time => "time",
            Dimension::Current => "current",
            Dimension::Potential => "potential",
            Dimension::Power => "power",
            Dimension::Resistance => "resistance",
            Dimension::Conductivity => "conductivity",
            Dimension::Temperature => "temperature",
            Dimension::Area => "area",
            Dimension::Volume => "volume",
            Dimension::Speed => "speed",
            Dimension::Acceleration => "acceleration",
            Dimension::TimeSquare => "time squared",
            Dimension::AbsoluteHumidity => "absolute humidity",
            Dimension::Frequency => "frequency",
            Dimension::FanSpeed => "fan speed",
            Dimension::Information => "information",
            Dimension::Operations => "operations",
            Dimension::Bandwidth => "bandwidth",
            Dimension::IOLatency => "i/o latency",
            Dimension::IOPerformance => "i/o performance",
            Dimension::AvgOpSize => "avg op size",
            Dimension::Dimensionless => "dimensionless",
        }
    }

    pub const fn reference_unit(&self) -> Unit {
        match self {
            Dimension::Length => Unit::Length(LengthUnit::REFERENCE),
            Dimension::Mass => Unit::Mass(MassUnit::REFERENCE),
            Dimension::Time => Unit::Time(TimeUnit::REFERENCE),
            Dimension::Current => Unit::Current(CurrentUnit::REFERENCE),
            Dimension::Potential => Unit::Potential(PotentialUnit::REFERENCE),
            Dimension::Power => Unit::Power(PowerUnit::REFERENCE),
            Dimension::Resistance => {
                Unit::Resistance(ResistanceUnit::REFERENCE)
            }
            Dimension::Conductivity => {
                Unit::Conductivity(ConductivityUnit::REFERENCE)
            }
            Dimension::Temperature => {
                Unit::Temperature(TemperatureUnit::Celsius)
            }
            Dimension::Area => Unit::Area(LengthUnit::REFERENCE),
            Dimension::Volume => Unit::Volume(LengthUnit::REFERENCE),
            Dimension::Speed => {
                Unit::Speed(LengthUnit::REFERENCE, TimeUnit::REFERENCE)
            }
            Dimension::Acceleration => {
                Unit::Acceleration(LengthUnit::REFERENCE, TimeUnit::REFERENCE)
            }
            Dimension::TimeSquare => Unit::TimeSquare(TimeUnit::REFERENCE),
            Dimension::AbsoluteHumidity => Unit::AbsoluteHumidity(
                MassUnit::REFERENCE,
                LengthUnit::REFERENCE,
            ),
            Dimension::Frequency => Unit::Frequency(FrequencyUnit::REFERENCE),
            Dimension::FanSpeed => Unit::FanSpeed(FanSpeedUnit::REFERENCE),
            Dimension::Information => {
                Unit::Information(InformationUnit::REFERENCE)
            }
            Dimension::Operations => Unit::Operations(OperationUnit::REFERENCE),
            Dimension::Bandwidth => Unit::Bandwidth(
                InformationUnit::Bit(DecPrefix::Unit),
                TimeUnit::REFERENCE,
            ),
            Dimension::IOLatency => {
                Unit::IOLatency(TimeUnit::REFERENCE, OperationUnit::REFERENCE)
            }
            Dimension::IOPerformance => Unit::IOPerformance(
                OperationUnit::REFERENCE,
                TimeUnit::REFERENCE,
            ),
            Dimension::AvgOpSize => Unit::AvgOpSize(
                InformationUnit::REFERENCE,
                OperationUnit::REFERENCE,
            ),
            Dimension::Dimensionless => {
                Unit::Dimensionless(DimensionlessUnit::REFERENCE)
            }
        }
    }

    pub const fn units(&self) -> &'static [Unit] {
        match self {
            Dimension::Length => &LENGTH_UNITS,
            Dimension::Mass => &MASS_UNITS,
            Dimension::Time => &TIME_UNITS,
            Dimension::Current => &CURRENT_UNITS,
            Dimension::Temperature => &TEMPERATURE_UNITS,
            Dimension::Area => &AREA_UNITS,
            Dimension::Volume => &VOLUME_UNITS,
            Dimension::Speed => &SPEED_UNITS,
            Dimension::Acceleration => &ACCELERATION_UNITS,
            Dimension::Potential => &POTENTIAL_UNITS,
            Dimension::Power => &POWER_UNITS,
            Dimension::Resistance => &RESISTANCE_UNITS,
            Dimension::Conductivity => &CONDUCTIVITY_UNITS,
            Dimension::AbsoluteHumidity => &ABSOLUTE_HUMIDITY_UNITS,
            Dimension::Frequency => &FREQUENCY_UNITS,
            Dimension::TimeSquare => &TIME_SQUARE_UNITS,
            Dimension::Information => &INFORMATION_UNITS,
            Dimension::Operations => &OPERATIONS_UNITS,
            Dimension::Bandwidth => &BANDWIDTH_UNITS,
            Dimension::IOLatency => &IO_LATENCY_UNITS,
            Dimension::IOPerformance => &IO_PERFORMANCE_UNITS,
            Dimension::AvgOpSize => &AVG_OP_SIZE_UNITS,
            Dimension::FanSpeed => &FAN_SPEED_UNITS,
            Dimension::Dimensionless => &DIMENSIONLESS_UNITS,
        }
    }
}

impl Display for Dimension {
    fn fmt(&self, f: &mut Formatter) -> Result<(), fmt::Error> {
        write!(f, "{}", self.name())
    }
}

/* Operations on dimensions, used for type-checking. */

impl Dimension {
    pub fn powi(self, n: i32) -> Result<Dimension, UnitError> {
        match (self, n) {
            (d, 1) => Ok(d),
            (_, 0) => Ok(Dimension::Dimensionless),
            (Dimension::Dimensionless, _) => Ok(Dimension::Dimensionless),
            (Dimension::Conductivity, -1) => Ok(Dimension::Resistance),
            (Dimension::Frequency, -2) => Ok(Dimension::TimeSquare),
            (Dimension::Frequency, -1) => Ok(Dimension::Time),
            (Dimension::IOLatency, -1) => Ok(Dimension::IOPerformance),
            (Dimension::IOPerformance, -1) => Ok(Dimension::IOLatency),
            (Dimension::Length, 2) => Ok(Dimension::Area),
            (Dimension::Length, 3) => Ok(Dimension::Volume),
            (Dimension::Resistance, -1) => Ok(Dimension::Conductivity),
            (Dimension::Time, -1) => Ok(Dimension::Frequency),
            (Dimension::Time, 2) => Ok(Dimension::TimeSquare),
            _ => Err(UnitError::Pow(self, n)),
        }
    }
}

impl Mul<Dimension> for Dimension {
    type Output = Result<Dimension, UnitError>;
    fn mul(self, rhs: Dimension) -> Result<Dimension, UnitError> {
        match (self, rhs) {
            (Dimension::Dimensionless, d) => Ok(d),
            (d, Dimension::Dimensionless) => Ok(d),
            (Dimension::AbsoluteHumidity, Dimension::Volume) => {
                Ok(Dimension::Mass)
            }
            (Dimension::Acceleration, Dimension::Time) => Ok(Dimension::Speed),
            (Dimension::Acceleration, Dimension::TimeSquare) => {
                Ok(Dimension::Length)
            }
            (Dimension::Area, Dimension::Length) => Ok(Dimension::Volume),
            (Dimension::AvgOpSize, Dimension::IOPerformance) => {
                Ok(Dimension::Bandwidth)
            }
            (Dimension::AvgOpSize, Dimension::Operations) => {
                Ok(Dimension::Information)
            }
            (Dimension::Bandwidth, Dimension::IOLatency) => {
                Ok(Dimension::AvgOpSize)
            }
            (Dimension::Bandwidth, Dimension::Time) => {
                Ok(Dimension::Information)
            }
            (Dimension::Conductivity, Dimension::Potential) => {
                Ok(Dimension::Current)
            }
            (Dimension::Conductivity, Dimension::Resistance) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Current, Dimension::Potential) => Ok(Dimension::Power),
            (Dimension::Current, Dimension::Resistance) => {
                Ok(Dimension::Potential)
            }
            (Dimension::Frequency, Dimension::Information) => {
                Ok(Dimension::Bandwidth)
            }
            (Dimension::Frequency, Dimension::Length) => Ok(Dimension::Speed),
            (Dimension::Frequency, Dimension::Operations) => {
                Ok(Dimension::IOPerformance)
            }
            (Dimension::Frequency, Dimension::Speed) => {
                Ok(Dimension::Acceleration)
            }
            (Dimension::Frequency, Dimension::Time) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Frequency, Dimension::TimeSquare) => {
                Ok(Dimension::Time)
            }
            (Dimension::IOLatency, Dimension::Bandwidth) => {
                Ok(Dimension::AvgOpSize)
            }
            (Dimension::IOLatency, Dimension::IOPerformance) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::IOLatency, Dimension::Operations) => {
                Ok(Dimension::Time)
            }
            (Dimension::IOPerformance, Dimension::AvgOpSize) => {
                Ok(Dimension::Bandwidth)
            }
            (Dimension::IOPerformance, Dimension::IOLatency) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::IOPerformance, Dimension::Time) => {
                Ok(Dimension::Operations)
            }
            (Dimension::Information, Dimension::Frequency) => {
                Ok(Dimension::Bandwidth)
            }
            (Dimension::Length, Dimension::Area) => Ok(Dimension::Volume),
            (Dimension::Length, Dimension::Frequency) => Ok(Dimension::Speed),
            (Dimension::Length, Dimension::Length) => Ok(Dimension::Area),
            (Dimension::Operations, Dimension::AvgOpSize) => {
                Ok(Dimension::Information)
            }
            (Dimension::Operations, Dimension::Frequency) => {
                Ok(Dimension::IOPerformance)
            }
            (Dimension::Operations, Dimension::IOLatency) => {
                Ok(Dimension::Time)
            }
            (Dimension::Potential, Dimension::Conductivity) => {
                Ok(Dimension::Current)
            }
            (Dimension::Potential, Dimension::Current) => Ok(Dimension::Power),
            (Dimension::Resistance, Dimension::Conductivity) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Resistance, Dimension::Current) => {
                Ok(Dimension::Potential)
            }
            (Dimension::Speed, Dimension::Frequency) => {
                Ok(Dimension::Acceleration)
            }
            (Dimension::Speed, Dimension::Time) => Ok(Dimension::Length),
            (Dimension::Time, Dimension::Acceleration) => Ok(Dimension::Speed),
            (Dimension::Time, Dimension::Bandwidth) => {
                Ok(Dimension::Information)
            }
            (Dimension::Time, Dimension::Frequency) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Time, Dimension::IOPerformance) => {
                Ok(Dimension::Operations)
            }
            (Dimension::Time, Dimension::Speed) => Ok(Dimension::Length),
            (Dimension::Time, Dimension::Time) => Ok(Dimension::TimeSquare),
            (Dimension::TimeSquare, Dimension::Acceleration) => {
                Ok(Dimension::Length)
            }
            (Dimension::TimeSquare, Dimension::Frequency) => {
                Ok(Dimension::Time)
            }
            (Dimension::Volume, Dimension::AbsoluteHumidity) => {
                Ok(Dimension::Mass)
            }
            _ => Err(UnitError::Mul(self, rhs)),
        }
    }
}

impl Div<Dimension> for Dimension {
    type Output = Result<Dimension, UnitError>;
    fn div(self, rhs: Dimension) -> Result<Dimension, UnitError> {
        match (self, rhs) {
            (d, Dimension::Dimensionless) => Ok(d),
            (Dimension::AbsoluteHumidity, Dimension::AbsoluteHumidity) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Acceleration, Dimension::Acceleration) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Acceleration, Dimension::Frequency) => {
                Ok(Dimension::Speed)
            }
            (Dimension::Acceleration, Dimension::Speed) => {
                Ok(Dimension::Frequency)
            }
            (Dimension::Area, Dimension::Area) => Ok(Dimension::Dimensionless),
            (Dimension::Area, Dimension::Length) => Ok(Dimension::Length),
            (Dimension::AvgOpSize, Dimension::AvgOpSize) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::AvgOpSize, Dimension::Bandwidth) => {
                Ok(Dimension::IOLatency)
            }
            (Dimension::AvgOpSize, Dimension::IOLatency) => {
                Ok(Dimension::Bandwidth)
            }
            (Dimension::Bandwidth, Dimension::AvgOpSize) => {
                Ok(Dimension::IOPerformance)
            }
            (Dimension::Bandwidth, Dimension::Bandwidth) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Bandwidth, Dimension::Frequency) => {
                Ok(Dimension::Information)
            }
            (Dimension::Bandwidth, Dimension::IOPerformance) => {
                Ok(Dimension::AvgOpSize)
            }
            (Dimension::Bandwidth, Dimension::Information) => {
                Ok(Dimension::Frequency)
            }
            (Dimension::Conductivity, Dimension::Conductivity) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Current, Dimension::Conductivity) => {
                Ok(Dimension::Potential)
            }
            (Dimension::Current, Dimension::Current) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Current, Dimension::Potential) => {
                Ok(Dimension::Conductivity)
            }
            (Dimension::Dimensionless, Dimension::Conductivity) => {
                Ok(Dimension::Resistance)
            }
            (Dimension::Dimensionless, Dimension::Frequency) => {
                Ok(Dimension::Time)
            }
            (Dimension::Dimensionless, Dimension::IOLatency) => {
                Ok(Dimension::IOPerformance)
            }
            (Dimension::Dimensionless, Dimension::IOPerformance) => {
                Ok(Dimension::IOLatency)
            }
            (Dimension::Dimensionless, Dimension::Resistance) => {
                Ok(Dimension::Conductivity)
            }
            (Dimension::Dimensionless, Dimension::Time) => {
                Ok(Dimension::Frequency)
            }
            (Dimension::FanSpeed, Dimension::FanSpeed) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Frequency, Dimension::Frequency) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::IOLatency, Dimension::IOLatency) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::IOPerformance, Dimension::Frequency) => {
                Ok(Dimension::Operations)
            }
            (Dimension::IOPerformance, Dimension::IOPerformance) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::IOPerformance, Dimension::Operations) => {
                Ok(Dimension::Frequency)
            }
            (Dimension::Information, Dimension::AvgOpSize) => {
                Ok(Dimension::Operations)
            }
            (Dimension::Information, Dimension::Bandwidth) => {
                Ok(Dimension::Time)
            }
            (Dimension::Information, Dimension::Information) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Information, Dimension::Operations) => {
                Ok(Dimension::AvgOpSize)
            }
            (Dimension::Information, Dimension::Time) => {
                Ok(Dimension::Bandwidth)
            }
            (Dimension::Length, Dimension::Acceleration) => {
                Ok(Dimension::TimeSquare)
            }
            (Dimension::Length, Dimension::Length) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Length, Dimension::Speed) => Ok(Dimension::Time),
            (Dimension::Length, Dimension::Time) => Ok(Dimension::Speed),
            (Dimension::Length, Dimension::TimeSquare) => {
                Ok(Dimension::Acceleration)
            }
            (Dimension::Mass, Dimension::AbsoluteHumidity) => {
                Ok(Dimension::Volume)
            }
            (Dimension::Mass, Dimension::Mass) => Ok(Dimension::Dimensionless),
            (Dimension::Mass, Dimension::Volume) => {
                Ok(Dimension::AbsoluteHumidity)
            }
            (Dimension::Operations, Dimension::IOPerformance) => {
                Ok(Dimension::Time)
            }
            (Dimension::Operations, Dimension::Operations) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Operations, Dimension::Time) => {
                Ok(Dimension::IOPerformance)
            }
            (Dimension::Potential, Dimension::Current) => {
                Ok(Dimension::Resistance)
            }
            (Dimension::Potential, Dimension::Potential) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Potential, Dimension::Resistance) => {
                Ok(Dimension::Current)
            }
            (Dimension::Power, Dimension::Current) => Ok(Dimension::Potential),
            (Dimension::Power, Dimension::Potential) => Ok(Dimension::Current),
            (Dimension::Power, Dimension::Power) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Resistance, Dimension::Resistance) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Speed, Dimension::Acceleration) => Ok(Dimension::Time),
            (Dimension::Speed, Dimension::Frequency) => Ok(Dimension::Length),
            (Dimension::Speed, Dimension::Length) => Ok(Dimension::Frequency),
            (Dimension::Speed, Dimension::Speed) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Speed, Dimension::Time) => Ok(Dimension::Acceleration),
            (Dimension::Temperature, Dimension::Temperature) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Time, Dimension::Frequency) => {
                Ok(Dimension::TimeSquare)
            }
            (Dimension::Time, Dimension::IOLatency) => {
                Ok(Dimension::Operations)
            }
            (Dimension::Time, Dimension::Operations) => {
                Ok(Dimension::IOLatency)
            }
            (Dimension::Time, Dimension::Time) => Ok(Dimension::Dimensionless),
            (Dimension::Time, Dimension::TimeSquare) => {
                Ok(Dimension::Frequency)
            }
            (Dimension::TimeSquare, Dimension::Time) => Ok(Dimension::Time),
            (Dimension::TimeSquare, Dimension::TimeSquare) => {
                Ok(Dimension::Dimensionless)
            }
            (Dimension::Volume, Dimension::Area) => Ok(Dimension::Length),
            (Dimension::Volume, Dimension::Length) => Ok(Dimension::Area),
            (Dimension::Volume, Dimension::Volume) => {
                Ok(Dimension::Dimensionless)
            }
            _ => Err(UnitError::Div(self, rhs)),
        }
    }
}

impl Add<Dimension> for Dimension {
    type Output = Result<Dimension, UnitError>;
    fn add(self, rhs: Dimension) -> Result<Dimension, UnitError> {
        match self == rhs {
            true => Ok(self),
            false => Err(UnitError::Conversion(self, rhs)),
        }
    }
}

impl Sub<Dimension> for Dimension {
    type Output = Result<Dimension, UnitError>;
    fn sub(self, rhs: Dimension) -> Result<Dimension, UnitError> {
        match self == rhs {
            true => Ok(self),
            false => Err(UnitError::Conversion(self, rhs)),
        }
    }
}
