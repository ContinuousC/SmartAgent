/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use crate::soap::SoapError;

use super::managed_entities::{self};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error during SOAP requests: {0}")]
    SoapError(#[from] SoapError),
    #[error("No hostname given with config")]
    NoHost,
    #[error("Unable to log in: {0}")]
    Login(SoapError),
    #[error("Entry not found in keyvault")]
    MissingKREntry,
    #[error("No {0} in KeyVault entry")]
    MissingKRObject(String),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Systemtime is before EPOCH")]
    SysTime,
    #[error("failed to generate request: {0}")]
    GenerateRequest(xml::writer::Error),
    #[error("failed to parse response (invalid xml): {0}")]
    ParseResponseXml(xml::reader::Error),
    #[error("failed to parse response: {0}")]
    ParseResponse(managed_entities::error::ParseError),
    #[error("{0}")]
    IpResolve(#[from] trust_dns_resolver::error::ResolveError),
    #[error("No Ip found for hostname: {0}")]
    NoIpFound(String),
}

#[derive(thiserror::Error, Debug)]
pub enum DTError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("Error during SOAP requests: {0}")]
    SoapError(#[from] SoapError),
    #[error("Error parsing string to integer: {0}")]
    ParseIntError(String),
    #[error("Systemtime is before EPOCH")]
    SysTime,
}

impl DTError {
    pub fn to_api(self) -> crate::error::DTError {
        crate::error::DTError::VMWare(self)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DTWarning {
    #[error("Field {0} not found")]
    FieldNotFound(String),
}
