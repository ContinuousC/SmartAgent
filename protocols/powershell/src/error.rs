/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("{0}")]
    WinRMError(#[from] winrm_rs::Error),
    #[error("No credentials given")]
    NoCredentials,
    #[error("Current credentials are not supported for this protocol")]
    UnsupportedCredentials,
    #[error("Entry not found in keyvault")]
    MissingKREntry,
    #[error("No {0} in KeyVault entry")]
    MissingKRObject(String),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("{0}")]
    Custom(String),
    #[error("{0}")]
    IpResolve(#[from] trust_dns_resolver::error::ResolveError),
    #[error("No Ip found for hostname: {0}")]
    NoIpFound(String),
    #[error("(0)")]
    Format(#[from] std::fmt::Error),
    #[error("Failed to access counterfile ({0}): {1}")]
    LoadCounters(PathBuf, #[source] std::io::Error),
    #[error("Error while connecting to windows agent: {0}")]
    WindowsAgent(#[from] windows_agent_client::Error),
}

pub type TypeResult<T> = std::result::Result<T, TypeError>;

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("Field {0} is missing enumvars")]
    EnumMissingVars(String),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
}

pub type DTWResult<T> = std::result::Result<T, DTWarning>;

#[derive(Error, Debug)]
pub enum DTWarning {}

pub type DTEResult<T> = std::result::Result<T, DTError>;

#[derive(Error, Debug)]
pub enum DTError {
    #[error("Error communicating with server while retrieving table: {0}")]
    Winrm(#[from] winrm_rs::Error),
    #[error("{0}")]
    PowerShell(#[from] Error),
    #[error("Command failed with code {0}: {1}")]
    CommandFailed(i32, String),
    #[error("Unable to deserialize csv: {0}")]
    CsvDeserialize(#[from] csv::Error),
    #[error("Unable to deserialize csv: {0}")]
    JsonDeserialize(#[from] serde_json::Error),
    #[error("Error while executing command on windows agent: {0}")]
    WindowsAgent(#[from] windows_agent_client::Error),
    #[error("Cannot parse WindowsAgent Output")]
    WindowsAgentOutput(#[source] serde_json::Error),
    #[error("failed to format script: {0}")]
    RenderError(#[from] handlebars::RenderError),
}
