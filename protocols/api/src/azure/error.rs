/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;

use rest_protocol::{RESTError, TemplateError};
use value::Type;

use crate::error::TypeError;

pub type Result<T> = std::result::Result<T, Error>;
pub type DTEResult<T> = std::result::Result<T, DTError>;

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
    #[error("{0}")]
    Azure(#[from] azure_protocol::AzureError),
}

#[derive(thiserror::Error, Debug)]
pub enum DTError {
    #[error("Error during rest request: {0}")]
    RESTError(#[from] RESTError),
    #[error("Error parsing template: {0}")]
    Template(#[from] TemplateError),
    #[error("Error during HTTP request: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Error deserializing JSON: {0}")]
    SerdeJsonError(#[from] serde_json::error::Error),
    #[error("Error compiling JsonPath {0}: {1}")]
    JsonPathError(String, String),
    #[error("{0}")]
    Azure(#[from] azure_protocol::AzureError),
}

impl DTError {
    pub fn to_api(self) -> crate::error::DTError {
        crate::error::DTError::Azure(self)
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
        crate::error::DTWarning::Azure(self)
    }
}
