/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    BinPrefix, DecPrefix, Dimension, DimensionlessUnit, FracPrefix,
    FrequencyUnit, InformationUnit, SiPrefix, TimeUnit, Unit, UnitError,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Units {
    Dimensionless(Vec<DimensionlessUnit>),
    Information(Vec<InformationUnit>),
    Bandwidth {
        #[serde(rename = "Information")]
        information: Vec<InformationUnit>,
        #[serde(rename = "Time")]
        time: TimeUnits,
    },
    Time(TimeUnits),
    Frequency(FrequencyUnits),
}

impl Units {
    pub fn from_unit_list(
        dimension: &Dimension,
        units: &Option<Vec<Unit>>,
        display_unit: &Option<Unit>,
    ) -> Result<Self, UnitError> {
        match dimension {
            Dimension::Dimensionless => Ok(Units::Dimensionless(match units {
                Some(units) => units
                    .iter()
                    .map(|unit| match unit {
                        Unit::Dimensionless(u) => Ok(*u),
                        _ => Err(UnitError::TypeError(*dimension, *unit)),
                    })
                    .collect::<Result<BTreeSet<_>, UnitError>>()?
                    .into_iter()
                    .collect(),
                None => match display_unit {
                    Some(Unit::Dimensionless(u)) => vec![*u],
                    _ => vec![DimensionlessUnit::Count(DecPrefix::Unit)],
                },
            })),
            Dimension::Information => Ok(Units::Information(match units {
                Some(units) => units
                    .iter()
                    .map(|unit| match unit {
                        Unit::Information(u) => Ok(*u),
                        _ => Err(UnitError::TypeError(*dimension, *unit)),
                    })
                    .collect::<Result<BTreeSet<_>, UnitError>>()?
                    .into_iter()
                    .collect(),
                None => vec![
                    InformationUnit::Bit(DecPrefix::Unit),
                    InformationUnit::Bit(DecPrefix::Kilo),
                    InformationUnit::Bit(DecPrefix::Mega),
                    InformationUnit::Bit(DecPrefix::Giga),
                    InformationUnit::Bit(DecPrefix::Tera),
                    InformationUnit::Byte(BinPrefix::Unit),
                    InformationUnit::Byte(BinPrefix::Kilo),
                    InformationUnit::Byte(BinPrefix::Mega),
                    InformationUnit::Byte(BinPrefix::Giga),
                    InformationUnit::Byte(BinPrefix::Tera),
                ],
            })),
            Dimension::Bandwidth => Ok(Units::Bandwidth {
                information: match units {
                    Some(units) => units
                        .iter()
                        .map(|unit| match unit {
                            Unit::Bandwidth(u, _) => Ok(*u),
                            _ => Err(UnitError::TypeError(*dimension, *unit)),
                        })
                        .collect::<Result<BTreeSet<_>, UnitError>>()?
                        .into_iter()
                        .collect(),
                    None => vec![
                        InformationUnit::Bit(DecPrefix::Unit),
                        InformationUnit::Bit(DecPrefix::Kilo),
                        InformationUnit::Bit(DecPrefix::Mega),
                        InformationUnit::Bit(DecPrefix::Giga),
                        InformationUnit::Bit(DecPrefix::Tera),
                        InformationUnit::Byte(BinPrefix::Unit),
                        InformationUnit::Byte(BinPrefix::Kilo),
                        InformationUnit::Byte(BinPrefix::Mega),
                        InformationUnit::Byte(BinPrefix::Giga),
                        InformationUnit::Byte(BinPrefix::Tera),
                    ],
                },
                time: TimeUnits::from_unit_list(dimension, units)?,
            }),
            Dimension::Time => {
                Ok(Units::Time(TimeUnits::from_unit_list(dimension, units)?))
            }
            Dimension::Frequency => {
                Ok(Units::Frequency(FrequencyUnits::from_unit_list(units)?))
            }
            _ => Err(UnitError::Unsupported(*dimension)),
        }
    }

    pub fn dimension(&self) -> Dimension {
        match self {
            Self::Dimensionless(_) => Dimension::Dimensionless,
            Self::Information(_) => Dimension::Information,
            Self::Bandwidth { .. } => Dimension::Bandwidth,
            Self::Time(_) => Dimension::Time,
            Self::Frequency(_) => Dimension::Frequency,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct TimeUnits(pub Vec<TimeUnit>);

impl TimeUnits {
    pub fn from_unit_list(
        dimension: &Dimension,
        units: &Option<Vec<Unit>>,
    ) -> Result<Self, UnitError> {
        match units {
            Some(units) => Ok(Self(
                units
                    .iter()
                    .map(|unit| match (dimension, unit) {
                        (Dimension::Time, Unit::Time(u)) => Ok(*u),
                        (Dimension::Bandwidth, Unit::Bandwidth(_, u)) => Ok(*u),
                        _ => Err(UnitError::TypeError(*dimension, *unit)),
                    })
                    .collect::<Result<BTreeSet<_>, UnitError>>()?
                    .into_iter()
                    .collect(),
            )),
            None => Ok(Self(vec![
                TimeUnit::Second(FracPrefix::Unit),
                TimeUnit::Minute,
                TimeUnit::Hour,
            ])),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(transparent)]
pub struct FrequencyUnits(pub Vec<FrequencyUnit>);

impl FrequencyUnits {
    pub fn from_unit_list(
        units: &Option<Vec<Unit>>,
    ) -> Result<Self, UnitError> {
        match units {
            Some(units) => Ok(Self(
                units
                    .iter()
                    .map(|unit| match unit {
                        Unit::Frequency(u) => Ok(*u),
                        _ => Err(UnitError::TypeError(
                            Dimension::Frequency,
                            *unit,
                        )),
                    })
                    .collect::<Result<BTreeSet<_>, UnitError>>()?
                    .into_iter()
                    .collect(),
            )),
            None => Ok(Self(vec![FrequencyUnit::Hertz(SiPrefix::Unit)])),
        }
    }
}
