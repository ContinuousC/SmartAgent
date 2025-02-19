/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::convert::TryInto;
use std::num::ParseIntError;

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use csv::ReaderBuilder;
use serde_json::Value as JsonValue;

use value::{EnumValue, IntEnumValue, Type, Value};

use crate::input::FieldSpec;
use crate::ms_graph::error::{DTEResult, DTWResult, DTWarning};

pub fn deserialize_csv(csv: String) -> DTEResult<Vec<HashMap<String, String>>> {
    let mut reader = ReaderBuilder::new()
        .delimiter(b',')
        .has_headers(true)
        .from_reader(csv.as_bytes());
    Ok(reader
        .deserialize()
        .map(|r| {
            let r: std::result::Result<HashMap<String, String>, _> = r;
            r
        })
        .collect::<std::result::Result<Vec<HashMap<String, String>>, csv::Error>>()?)
}

pub fn parse_dt(val: &str) -> DTWResult<DateTime<Utc>> {
    let datevec = val
        .split('-')
        .map(|s| s.parse::<i32>())
        .collect::<std::result::Result<Vec<i32>, ParseIntError>>()?;
    if datevec.len() < 3 {
        return Err(DTWarning::ParseError(Type::Time, val.to_string()));
    }
    Ok(Utc.from_utc_datetime(
        &NaiveDate::from_ymd_opt(
            datevec[0],
            datevec[1].try_into()?,
            datevec[2].try_into()?,
        )
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap(),
    ))
}

pub fn parse_val(field: &FieldSpec, val: &String) -> DTWResult<Value> {
    let typ = field.get_type()?;
    let parse_err = || DTWarning::ParseError(typ.clone(), val.clone());
    match &typ {
        Type::Float => Ok(Value::Float(if val.is_empty() {
            0.0
        } else {
            val.parse().map_err(|_| parse_err())?
        })),
        Type::Integer => Ok(Value::Integer(if val.is_empty() {
            0
        } else {
            val.parse().map_err(|_| parse_err())?
        })),
        Type::UnicodeString => Ok(Value::UnicodeString(val.to_string())),
        Type::Enum(vals) => Ok(Value::Enum(
            EnumValue::new(vals.clone(), val.to_string())
                .map_err(|_| parse_err())?,
        )),
        Type::IntEnum(vals) => Ok(Value::IntEnum(
            IntEnumValue::new(
                vals.clone(),
                val.parse().map_err(|_| parse_err())?,
            )
            .map_err(|_| parse_err())?,
        )),
        Type::Boolean => Ok(Value::Boolean(if val.is_empty() {
            false
        } else {
            val.to_lowercase().parse().map_err(|_| parse_err())?
        })),
        Type::Time => Ok(Value::Time(parse_dt(val)?)),
        _ => Err(DTWarning::UnSupportedType(field.get_type()?)),
    }
}

pub fn parse_jsonval(field: &FieldSpec, val: JsonValue) -> DTWResult<Value> {
    let typ = field.get_type()?;
    let parse_err = || DTWarning::ParseError(typ.clone(), val.to_string());
    match &typ {
        Type::Float => Ok(Value::Float(
            serde_json::from_value(val.clone()).map_err(|_| parse_err())?,
        )),
        Type::Integer => Ok(Value::Integer(
            serde_json::from_value(val.clone()).map_err(|_| parse_err())?,
        )),
        Type::UnicodeString => {
            Ok(Value::UnicodeString(if let Some(xs) = val.as_array() {
                xs.iter()
                    .map(|s| {
                        s.as_str().ok_or(DTWarning::ParseError(
                            typ.clone(),
                            val.to_string(),
                        ))
                    })
                    .collect::<DTWResult<Vec<&str>>>()?
                    .join(", ")
            } else {
                serde_json::from_value(val.clone()).map_err(|_| parse_err())?
            }))
        }
        Type::Enum(vals) => Ok(Value::Enum(
            EnumValue::new(
                vals.clone(),
                serde_json::from_value(val.clone()).map_err(|_| parse_err())?,
            )
            .map_err(|_| parse_err())?,
        )),
        Type::IntEnum(vals) => Ok(Value::IntEnum(
            IntEnumValue::new(
                vals.clone(),
                serde_json::from_value(val.clone()).map_err(|_| parse_err())?,
            )
            .map_err(|_| parse_err())?,
        )),
        Type::Boolean => Ok(Value::Boolean(
            serde_json::from_value(val.clone()).map_err(|_| parse_err())?,
        )),
        Type::Time => Ok(Value::Time(
            serde_json::from_value(val.clone()).map_err(|_| parse_err())?,
        )),
        _ => Err(DTWarning::UnSupportedType(field.get_type()?)),
    }
}
