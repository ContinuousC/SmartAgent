/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use crate::input::WmiType;

pub type Result<T> = std::result::Result<T, WMIError>;
pub type DTResult<T> = std::result::Result<T, WMIDTError>;
pub type TypeResult<T> = std::result::Result<T, TypeError>;

#[derive(thiserror::Error, Debug)]
pub enum WMIError {
    #[error("Winrm Error: {0}")]
    WinRMError(#[from] winrm_rs::Error),
    #[error("{0}")]
    WinRMProtError(#[from] powershell_protocol::Error),
    #[error("(0)")]
    Format(#[from] std::fmt::Error),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("Entry not found in keyvault")]
    MissingKREntry,
    #[error("No {0} in KeyVault entry")]
    MissingKRObject(String),
    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Unable to (de)serialize: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Unable to retrieve the timezone of the server: {0}")]
    LocalTimezone(String),
    #[error("Unable to parse timezone {0}: {1}")]
    TimeZoneParse(String, String),
    #[error("{0}")]
    Custom(String),
    #[error("Using Dcom requires powershell-ntlm credentials")]
    InvalidDcomConfig,
    #[error("Authfile is missing the follwing entry: {0}")]
    MissingInAuthfile(String),
    #[error("Host has not connection configuration")]
    NoConnectionConfig,
    #[error("{0}")]
    PowerShell(#[from] powershell_protocol::DTError),
    #[error("could not read passwordfile with sudo: {0}")]
    ReadSudofile(#[source] std::io::Error),
    #[error("error resolving hostname: {0}")]
    DnsResolve(#[from] trust_dns_resolver::error::ResolveError),
}

#[derive(thiserror::Error, Debug)]
pub enum WMIDTError {
    #[error("Winrm Error: {0}")]
    WinRMError(#[from] winrm_rs::Error),
    #[error("{0}")]
    Powershell(#[from] powershell_protocol::DTError),
    #[error("{0}")]
    Request(String),
    #[error("Unable to create socket pair for wmic command: {0}")]
    SocketCreation(#[source] std::io::Error),
    #[error("Unable to write password over socket to wmic: {0}")]
    WritePassword(#[source] std::io::Error),
    #[error("Unable to execute wmic: {0}")]
    ExecuteWmic(#[source] std::io::Error),
    #[error("wmic query timed out")]
    WmicTimeout,
    #[error("Unable to query using wmic: {0}")]
    QueryWmic(String),
    #[error("Unable to spawn wmic: {0}")]
    SpawnWmic(#[source] std::io::Error),
    #[error("output from wmic is not valid utf-8: {0}")]
    ParseUTF8(#[from] std::string::FromUtf8Error),
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum TypeError {
    #[error("Unable to parse {0:?}: {1}")]
    ParseError(WmiType, String),
    #[error("(0)")]
    Format(#[from] std::fmt::Error),
}
