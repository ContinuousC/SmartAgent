/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::de::Error;
use serde::ser::SerializeMap;
use serde::{de, ser};
use std::collections::BTreeMap;
use std::fmt;

pub(crate) struct Visitor;

pub fn serialize<S>(
    map: &BTreeMap<i64, String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    let mut m = serializer.serialize_map(Some(map.len()))?;
    for (k, v) in map {
        m.serialize_entry(&k.to_string(), v)?;
    }
    m.end()
}

pub fn deserialize<'de, D>(
    deserializer: D,
) -> Result<BTreeMap<i64, String>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_map(Visitor)
}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = BTreeMap<i64, String>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an int -> string map")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        let mut m = BTreeMap::new();
        while let Some((k, v)) = map.next_entry::<String, String>()? {
            m.insert(
                k.parse().map_err(|_| {
                    A::Error::invalid_value(de::Unexpected::Str(&k), &self)
                })?,
                v,
            );
        }
        Ok(m)
    }
}
