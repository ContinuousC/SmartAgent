/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::de::{DeserializeSeed, Deserializer, Error};
use serde::Deserialize;

use super::{Dimension, DimensionlessUnit, InformationUnit, TimeUnit, Unit};

pub struct UnitSeed(pub Dimension);

impl<'de> DeserializeSeed<'de> for UnitSeed {
    type Value = Unit;
    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        match &self.0 {
            Dimension::Dimensionless => Ok(Unit::Dimensionless(
                DimensionlessUnit::deserialize(deserializer)?,
            )),
            Dimension::Information => Ok(Unit::Information(
                InformationUnit::deserialize(deserializer)?,
            )),
            Dimension::Bandwidth => {
                match BandwidthUnit::deserialize(deserializer)? {
                    BandwidthUnit::Tuple(information, time) => {
                        Ok(Unit::Bandwidth(information, time))
                    }
                    BandwidthUnit::Object { information, time } => {
                        Ok(Unit::Bandwidth(information, time))
                    }
                }
            }
            _ => Err(D::Error::custom("Unimplemented dimension")),
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BandwidthUnit {
    Tuple(InformationUnit, TimeUnit),
    Object {
        information: InformationUnit,
        time: TimeUnit,
    },
}
