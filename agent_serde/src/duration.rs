/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use chrono::Duration;
use serde::{de, ser};
/// Serialize durations to/from f64 number of seconds. Duration is implemented
/// as a structure with an i64 seconds and an i32 nanosecond field, but for some
/// reason is limited to the i64 range in milliseconds. Even though a double
/// precision float cannot accurately represent the full range, it should be
/// enough for all practical purposes, and makes for easier interaction with
/// the outside world. The double precision float has nanosecond precision up
/// to a little more than 292 years (positive or negative), microsecond precision
/// up to 292471 years (positive or negative) and millisecond precision for the
/// rest of the available range (-292471209 years to 292471209 years).
use std::fmt;

struct Visitor;

pub fn serialize<S>(
    duration: &Duration,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: ser::Serializer,
{
    match duration.num_nanoseconds() {
        Some(n) => serializer.serialize_f64(n as f64 * 10.0f64.powi(-9)),
        None => match duration.num_microseconds() {
            Some(n) => serializer.serialize_f64(n as f64 * 10.0f64.powi(-6)),
            None => serializer.serialize_f64(
                duration.num_milliseconds() as f64 * 10.0f64.powi(-3),
            ),
        },
    }
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: de::Deserializer<'de>,
{
    deserializer.deserialize_f64(Visitor)
    //.or(deserializer.deserialize_i64(Visitor))
}

fn to_i64(value: f64) -> Option<i64> {
    if value > i64::MIN as f64 && value < i64::MAX as f64 {
        Some(value as i64)
    } else {
        None
    }
}

impl<'de> de::Visitor<'de> for Visitor {
    type Value = Duration;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a duration in seconds")
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Duration, E> {
        if let Some(val) = to_i64(value * 10f64.powi(9)) {
            Ok(Duration::nanoseconds(val))
        } else if let Some(val) = to_i64(value * 10f64.powi(6)) {
            Ok(Duration::microseconds(val))
        } else if let Some(val) = to_i64(value * 10f64.powi(3)) {
            Ok(Duration::milliseconds(val))
        } else {
            Err(E::invalid_value(de::Unexpected::Float(value), &self))
        }
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Duration, E> {
        Ok(Duration::seconds(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Duration, E> {
        Ok(Duration::seconds(value as i64))
    }
}
