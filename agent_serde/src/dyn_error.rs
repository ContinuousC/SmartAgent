/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{de, ser};
use std::fmt;

struct Visitor;

pub fn serialize<S>(
    err: &Box<dyn std::error::Error + Send + Sync + 'static>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    serializer.serialize_str(&err.to_string())
}

pub fn deserialize<'de, D>(
    deserializer: D,
) -> Result<Box<dyn std::error::Error + Send + Sync + 'static>, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_string(Visitor)
}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Box<dyn std::error::Error + Send + Sync + 'static>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "an error string")
    }

    fn visit_string<E>(self, err: String) -> Result<Self::Value, E>
    where
        E: std::error::Error,
    {
        Ok(err.into())
    }
    fn visit_str<E>(self, err: &str) -> Result<Self::Value, E>
    where
        E: std::error::Error,
    {
        Ok(err.into())
    }
}
