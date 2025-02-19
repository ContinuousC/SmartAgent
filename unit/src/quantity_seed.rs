/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt;

use serde::de::{DeserializeSeed, Deserializer, Error, MapAccess, Visitor};

use super::{Dimension, Quantity, Unit, UnitSeed};

pub struct QuantitySeed(pub Dimension);

impl<'de> DeserializeSeed<'de> for QuantitySeed {
    type Value = Quantity;
    fn deserialize<D: Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> Result<Self::Value, D::Error> {
        deserializer.deserialize_struct("Quantity", &["value", "unit"], self)
    }
}

impl<'de> Visitor<'de> for QuantitySeed {
    type Value = Quantity;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "A Quantity object")
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut value: Option<f64> = None;
        let mut unit: Option<Unit> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "value" => value = Some(map.next_value()?),
                "unit" => unit = Some(map.next_value_seed(UnitSeed(self.0))?),
                _ => continue,
            }
        }
        Ok(Quantity(
            value.ok_or_else(|| A::Error::missing_field("value"))?,
            unit.ok_or_else(|| A::Error::missing_field("unit"))?,
        ))
    }
}
