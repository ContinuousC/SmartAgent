/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use chrono::{DateTime, Utc};
use etc_base::ProtoDataFieldId;
use protocol::CounterDb;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    sync::Arc,
};

use std::sync::Mutex;
use value::{Data, DataError, EnumValue, IntEnumValue, Value};

use crate::input::FieldSpec;

use super::parse_val;

// An enum for all types off requests,
// the request url can be constructed from this enum because the display trait is implemented,
// and this is later matched to be able to pass the correct body type to the parse_xml function
#[derive(Debug, Deserialize)]
pub enum Requests {
    GetSystem,
    GetRoutine,
    GetGlobal,
    GetECPDataSvr,
    GetECPAppSvr,
    GetDashboard,
    EnumBuffer,
    EnumDatabase,
    EnumProcess,
    EnumWriteDaemon,
    EnumResource,
}

impl Display for Requests {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Requests::GetSystem => "GetSystem",
                Requests::GetRoutine => "GetRoutine",
                Requests::GetGlobal => "GetGlobal",
                Requests::GetECPDataSvr => "GetECPDataSvr",
                Requests::GetECPAppSvr => "GetECPAppSvr",
                Requests::GetDashboard => "GetDashboard",
                Requests::EnumBuffer => "EnumBuffer",
                Requests::EnumDatabase => "EnumDatabase",
                Requests::EnumProcess => "EnumProcess",
                Requests::EnumWriteDaemon => "EnumWriteDaemon",
                Requests::EnumResource => "EnumResource",
            }
        )
    }
}

// Generic struct for "lowest" and "highest" level.
// All levels in between have specific names and cannot be generalized.
#[derive(Debug, Deserialize, Clone)]
pub struct ValueSoap<T> {
    #[serde(rename = "$text")]
    pub value: T,
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    #[serde(rename = "Body")]
    pub body: T,
}

pub fn form_enum(
    field: &FieldSpec,
    item_value: &str,
) -> Result<Value, DataError> {
    if let Some(values) = field.values.clone() {
        match values {
            crate::input::ValueTypes::Integer(_) => Err(DataError::TypeError(
                "Expected enum string values".to_string(),
            )),
            crate::input::ValueTypes::String(content) => {
                let t = EnumValue::new(content.clone(), item_value.to_string());
                match t {
                    Ok(enumvalue) => Ok(Value::Enum(enumvalue)),
                    Err(e) => Err(e),
                }
            }
        }
    } else {
        Err(DataError::TypeError("Expected enum values".to_string()))
    }
}

pub fn form_int_enum(
    field: &FieldSpec,
    item_value: i64,
) -> Result<Value, DataError> {
    if let Some(values) = field.values.clone() {
        match values {
            crate::input::ValueTypes::Integer(content) => {
                let t = IntEnumValue::new(content, item_value);
                match t {
                    Ok(enumvalue) => Ok(Value::IntEnum(enumvalue)),
                    Err(e) => Err(e),
                }
            }
            crate::input::ValueTypes::String(_) => Err(DataError::TypeError(
                "Expected enum int values".to_string(),
            )),
        }
    } else {
        Err(DataError::TypeError("Expected enum values".to_string()))
    }
}

pub trait CreateTabledata {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>>;
}

pub fn create_string_data(
    id: &ProtoDataFieldId,
    option: &Option<ValueSoap<String>>,
) -> (ProtoDataFieldId, Result<Value, DataError>) {
    (
        id.clone(),
        option
            .as_ref()
            .map(|value| Value::UnicodeString(value.value.clone()))
            .ok_or(DataError::Missing),
    )
}

pub fn create_int_data(
    id: &ProtoDataFieldId,
    option: &Option<ValueSoap<i64>>,
) -> (ProtoDataFieldId, Result<Value, DataError>) {
    (
        id.clone(),
        option
            .as_ref()
            .map(|value| Value::Integer(value.value))
            .ok_or(DataError::Missing),
    )
}

pub fn create_float_data(
    id: &ProtoDataFieldId,
    option: &Option<ValueSoap<f64>>,
) -> (ProtoDataFieldId, Result<Value, DataError>) {
    (
        id.clone(),
        option
            .as_ref()
            .map(|value| Value::Float(value.value))
            .ok_or(DataError::Missing),
    )
}

pub fn create_bool_data(
    id: &ProtoDataFieldId,
    option: &Option<ValueSoap<bool>>,
) -> (ProtoDataFieldId, Result<Value, DataError>) {
    (
        id.clone(),
        option
            .as_ref()
            .map(|value| Value::Boolean(value.value))
            .ok_or(DataError::Missing),
    )
}

pub fn create_int_enum_data(
    id: &ProtoDataFieldId,
    option: &Option<ValueSoap<i64>>,
    field: &&FieldSpec,
) -> (ProtoDataFieldId, Result<Value, DataError>) {
    (
        id.clone(),
        option
            .as_ref()
            .map(|value| form_int_enum(field, value.value))
            .ok_or(DataError::Missing)
            .and_then(|a| a),
    )
}

pub fn create_enum_data(
    id: &ProtoDataFieldId,
    option: &Option<ValueSoap<String>>,
    field: &&FieldSpec,
) -> (ProtoDataFieldId, Result<Value, DataError>) {
    (
        id.clone(),
        option
            .as_ref()
            .map(|value| form_enum(field, &value.value))
            .ok_or(DataError::Missing)
            .and_then(|a| a),
    )
}

pub fn create_time_data(
    id: &ProtoDataFieldId,
    option: &Option<ValueSoap<DateTime<Utc>>>,
) -> (ProtoDataFieldId, Result<Value, DataError>) {
    (
        id.clone(),
        option
            .as_ref()
            .map(|value| Value::Time(value.value))
            .ok_or(DataError::Missing),
    )
}

pub fn create_data_with_counter_db(
    id: &ProtoDataFieldId,
    key: String,
    option: &Option<ValueSoap<u64>>,
    counterdb: &Arc<Mutex<CounterDb>>,
    field: &&FieldSpec,
) -> (ProtoDataFieldId, Result<Value, DataError>) {
    (
        id.clone(),
        option
            .as_ref()
            .map(|value| parse_val(field, counterdb, key, value.value))
            .ok_or(DataError::Missing)
            .and_then(|a| a),
    )
}
