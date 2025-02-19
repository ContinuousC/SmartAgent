/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;

use crate::error;
pub type Result<T> = std::result::Result<T, Error>;

pub type DTResult<T> = std::result::Result<T, DTError>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cannot get data from API: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Cannot login")]
    Authentication,
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("Failed to read certificate: {0}")]
    IO(#[from] std::io::Error),
    #[error("Failed to parse certificate: {0}")]
    CertParse(#[source] reqwest::Error),
    #[error("Failed to save counter database: {0}")]
    CounterDbSave(#[source] std::io::Error),
    #[error("{0}")]
    KeyReader(agent_utils::Error),
    #[error("Credentials not found in keyvault ({0})")]
    CredentialsNotFound(String),
    #[error("{0}")]
    Custom(String),
    #[error("{0}")]
    IpResolve(#[from] trust_dns_resolver::error::ResolveError),
    #[error("No Ip found for hostname: {0}")]
    NoIpFound(String),
}
#[derive(Debug, Error)]
pub enum DTWarning {}

#[derive(Debug, Error)]
pub enum DTError {
    #[error("Cannot get data from API: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("Unknown command: {0}")]
    UnknownCommand(String),
    #[error("Xml parsing error: {0} ({1})")]
    ParseXml(#[source] quick_xml::DeError, String),
    #[error("No data found for field {0}")]
    MissingData(String),
    #[error("Post failed: {0}")]
    Post(#[source] error::Error),
    #[error("Login attempt failed: {0}")]
    Cache(Error),
}
