/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;

use etc_base::ProtoDataTableId;
use rest_protocol::{RESTError, TemplateError};

use crate::definitions::ErrorResponse;

pub type Result<T> = std::result::Result<T, AzureError>;

#[derive(Error, Debug)]
pub enum AzureError {
    #[error("Error during rest request: {0}")]
    RESTError(#[from] RESTError),
    #[error("Error While parsing template: {0}")]
    TemplateError(#[from] TemplateError),
    #[error("Error deserializing JSON: {0}")]
    SerdeJsonError(#[from] serde_json::error::Error),
    #[error("Error from response: {0}")]
    ResponseError(String),
    #[error("No ClientSecret or ClientName given")]
    NoPassword,
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Error during formating: {0}")]
    FmtError(#[from] std::fmt::Error),
    #[error("No credentials given")]
    NoLogin,
    #[error("Recieved an error from azure: {}", .0.error.message)]
    Response(ErrorResponse),
}

#[derive(Error, Debug)]
pub enum AzureDataError {
    #[error("Error during rest request {0}: {1}")]
    RESTError(ProtoDataTableId, RESTError),
    #[error("Error While parsing template {0}: {1}")]
    TemplateError(ProtoDataTableId, TemplateError),
    #[error("Error deserializing JSON {0}: {1}")]
    SerdeJsonError(ProtoDataTableId, serde_json::error::Error),
    #[error("Error from response {0}: {1}")]
    ResponseError(ProtoDataTableId, String),
    #[error("Error retrieving data from azure {0}: {1}")]
    AzureData(ProtoDataTableId, AzureError),
}

impl AzureDataError {
    pub fn get_dt(&self) -> ProtoDataTableId {
        match self {
            Self::RESTError(dt, _) => dt,
            Self::TemplateError(dt, _) => dt,
            Self::SerdeJsonError(dt, _) => dt,
            Self::ResponseError(dt, _) => dt,
            Self::AzureData(dt, _) => dt,
        }
        .clone()
    }
}
