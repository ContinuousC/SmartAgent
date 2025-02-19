/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;

use super::value::Value;
use serde::{Deserialize, Serialize};

pub type Data = std::result::Result<Value, DataError>;

#[derive(Serialize, Deserialize, Error, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum DataError {
    #[error("Missing data")]
    Missing,
    #[error("Unhashable value in set or map")]
    Unhashable,
    #[error("Invalid enum choice: {0}")]
    InvalidChoice(String),
    #[error("Invalid int enum choice: {0}")]
    InvalidIntChoice(i64),
    #[error("Wrong data type: {0}")]
    TypeError(String),
    #[error("Counter overflow")]
    CounterOverflow,
    #[error("Counter pending")]
    CounterPending,
    #[error("Counter undefined")]
    CounterUndefined,
    #[error("Invalid mac address")]
    InvalidMacAddress(String),
    #[error("Invalid ipv4 address")]
    InvalidIpv4Address(String),
    #[error("Invalid ipv6 address")]
    InvalidIpv6Address(String),
    #[error("Invalid option value")]
    InvalidOptionValue,
    #[error("Invalid result value")]
    InvalidResultValue,
    #[error("Invalid set value")]
    InvalidSetValue,
    #[error("Invalid list value")]
    InvalidListValue,
    #[error("Json error")]
    Json(String),
    #[error("Unit conversion error: {0}")]
    UnitConversion(#[from] unit::UnitError),
    #[error("{0}")]
    External(String),
    #[error("Failed to parse value {0:?} to type {1}")]
    Parse(String, String),
}
