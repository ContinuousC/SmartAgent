/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::de::Deserializer;
use serde::ser::{Error, Serializer};
use serde::{Deserialize, Serialize};

use crate::{DimensionlessUnit, InformationUnit, TimeUnit, Unit};

pub fn serialize<S: Serializer>(
    unit: &Unit,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match *unit {
        Unit::Dimensionless(unit) => UnitAsObject::Dimensionless(unit),
        Unit::Information(unit) => UnitAsObject::Information(unit),
        Unit::Bandwidth(information, time) => {
            UnitAsObject::Bandwidth { information, time }
        }
        _ => {
            return Err(S::Error::custom(format!(
                "Unimplemented UnitAsObject: {}",
                unit
            )))
        }
    }
    .serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Unit, D::Error> {
    Ok(match UnitAsObject::deserialize(deserializer)? {
        UnitAsObject::Dimensionless(unit) => Unit::Dimensionless(unit),
        UnitAsObject::Information(unit) => Unit::Information(unit),
        UnitAsObject::Bandwidth { information, time } => {
            Unit::Bandwidth(information, time)
        }
    })
}

#[derive(Serialize, Deserialize)]
enum UnitAsObject {
    Dimensionless(DimensionlessUnit),
    Information(InformationUnit),
    Bandwidth {
        #[serde(rename = "Information")]
        information: InformationUnit,
        #[serde(rename = "Time")]
        time: TimeUnit,
    },
}
