/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde_json::Value as JsonValue;
use thiserror::Error;

use rest_protocol::{RESTError, TemplateError};
use value::Type;

use crate::error::TypeError;

pub type Result<T> = std::result::Result<T, Error>;
pub type DTEResult<T> = std::result::Result<T, DTError>;
pub type DTWResult<T> = std::result::Result<T, DTWarning>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Error during rest request: {0}")]
    RESTError(#[from] RESTError),
    #[error("Error While parsing template: {0}")]
    TemplateError(#[from] TemplateError),
    #[error("Error deserializing JSON: {0}")]
    SerdeJsonError(#[from] serde_json::error::Error),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Error during formating: {0}")]
    FmtError(#[from] std::fmt::Error),
    #[error("{0}")]
    DTError(#[from] DTError),
    #[error("Error during HTTP request: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("No ClientSecret or ClientName given")]
    NoPassword,
}

impl Error {
    pub fn to_api(self) -> crate::error::Error {
        crate::error::Error::MSGraph(self)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DTError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Error during rest request: {0}")]
    RESTError(#[from] RESTError),
    #[error("Error during HTTP request: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("unable to deserialize csv: {0}")]
    CsvError(#[from] csv::Error),
    #[error("Error deserializing JSON: {0}")]
    SerdeJsonError(#[from] serde_json::error::Error),
    #[error("Error compiling JsonPath {0}: {1}")]
    JsonPathError(String, String),
    #[error("Cannot find the id with path {0} in object {1}")]
    IdNotFound(String, JsonValue),
    #[error("Request to '{0}' failed: many retries")]
    ToManyRetries(String),
    #[error("Time went backwards")]
    SystemTimeError,
    #[error("Cannot request data from url: {0}. Check if the permissions of the user are correct")]
    Forbidden(String),
    #[error("{0}")]
    EtcSyntaxError(String),
    #[error("cannot parse json to an {1}: {0}")]
    ParseJsonObject(JsonValue, String),
}

impl DTError {
    pub fn to_api(self) -> crate::error::DTError {
        crate::error::DTError::MSGraph(self)
    }
    pub fn to_err(self) -> Error {
        Error::DTError(self)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DTWarning {
    #[error("Field not found in response: {0}")]
    FieldNotFound(String),
    #[error("Parsing of type {0} not (yet) supported")]
    UnSupportedType(Type),
    #[error("Cannot parse '{1}' to {0}")]
    ParseError(Type, String),
    #[error("Cannot get type: {0}")]
    TypeError(#[from] TypeError),
    #[error("Cannot parse datatime from '{0}' (expected: '%Y-%m-%d')")]
    ParseDTError(#[from] chrono::ParseError),
    #[error("Cannot parse to int: '{0}'")]
    ParseIntError(#[from] std::num::ParseIntError),
    #[error("Cannot parse to int: '{0}'")]
    TryFromIntError(#[from] std::num::TryFromIntError),
}

impl DTWarning {
    pub fn to_api(self) -> crate::error::DTWarning {
        crate::error::DTWarning::MSGraph(self)
    }
}
