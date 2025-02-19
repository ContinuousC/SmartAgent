/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use regex::Regex;
use serde::{de, ser};
use std::fmt;

struct Visitor;

pub fn serialize<S>(regex: &Regex, serializer: S) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    serializer.serialize_str(regex.as_str())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_str(Visitor)
}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Regex;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a valid regular expression string")
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Regex, E> {
        match Regex::new(value) {
            Ok(regex) => Ok(regex),
            Err(_) => Err(E::invalid_value(de::Unexpected::Str(value), &self)),
        }
    }
}

pub fn compare(a: &Regex, b: &Regex) -> bool {
    a.as_str() == b.as_str()
}
