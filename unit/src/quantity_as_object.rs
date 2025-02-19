/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::de::Deserializer;
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

use crate::unit_as_object;
use crate::{DecPrefix, DimensionlessUnit, Quantity, Unit};

#[derive(Serialize, Deserialize)]
struct QuantityObject {
    value: f64,
    #[serde(with = "unit_as_object")]
    unit: Unit,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
enum QuantityEnum {
    Number(f64),
    Tuple(Quantity),
    Object(QuantityObject),
}

impl From<Quantity> for QuantityObject {
    fn from(val: Quantity) -> Self {
        QuantityObject {
            value: val.0,
            unit: val.1,
        }
    }
}

impl From<QuantityEnum> for Quantity {
    fn from(val: QuantityEnum) -> Self {
        match val {
            QuantityEnum::Number(v) => Quantity(
                v,
                Unit::Dimensionless(DimensionlessUnit::Count(DecPrefix::Unit)),
            ),
            QuantityEnum::Tuple(q) => q,
            QuantityEnum::Object(q) => q.into(),
        }
    }
}

impl From<QuantityObject> for Quantity {
    fn from(val: QuantityObject) -> Self {
        Quantity(val.value, val.unit)
    }
}

pub fn serialize<S: Serializer>(
    quantity: &Quantity,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let object: QuantityObject = (*quantity).into();
    object.serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Quantity, D::Error> {
    Ok(QuantityEnum::deserialize(deserializer)?.into())
}
