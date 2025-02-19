/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::process::ExitStatus;

pub type Result<T> = std::result::Result<T, Error>;
pub type DTEResult<T> = std::result::Result<T, DTError>;
pub type DTWResult<T> = std::result::Result<T, DTWarning>;
pub type ApiResult<T> = std::result::Result<T, ApiError>;
pub type SmbResult<T> = std::result::Result<T, SmbError>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Entry not found in keyvault")]
    MissingKREntry,
    #[error("No {0} in KeyVault entry")]
    MissingKRObject(String),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),

    #[error("{0}")]
    Api(#[from] ApiError),
    #[error("{0}")]
    Smb(#[from] SmbError),
}

#[derive(thiserror::Error, Debug)]
pub enum DTError {
    #[error("{0}")]
    Api(#[from] ApiError),
    #[error("{0}")]
    Smb(#[from] SmbError),
}

#[derive(thiserror::Error, Debug)]
pub enum DTWarning {
    #[error("{0}")]
    Api(#[from] ApiError),
    #[error("{0}")]
    Smb(#[from] SmbError),
    #[error("{0}")]
    DTError(#[from] DTError),
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Credentials(#[from] protocol::auth::Error),
    #[error("No credentials were provided")]
    NoCredentials,
    #[error("unable to build a http client: {0}")]
    BuildApiClient(#[from] protocol::http::Error),
    #[error("unable to send request to api: {0} (0:?)")]
    SendRequest(#[source] reqwest::Error),
    #[error("unable to get the body of the response: {0}")]
    RetrieveBody(#[source] reqwest::Error),
    #[error("received an invalid response from the api: {1} {0}")]
    InvalidResponse(String, #[source] reqwest::Error),
    #[error("could not deserialize the body of the response: {0}")]
    DeserializeBody(#[source] serde_xml_rs::Error),
    #[error("{0}")]
    Plugin(#[from] Box<Error>),
    #[error("{0}")]
    Keyvault(#[from] crate::config::KeyvaultError),
}

#[derive(Debug, thiserror::Error)]
pub enum SmbError {
    #[error("no password was found for user {0}")]
    NoPassword(String),
    #[error("Invalid SmbPath ({0}): {1}")]
    InvalidSmbPath(String, String),
    #[error("illegal characters in smb path: {0}")]
    IllegalCharacters(String),
    #[error("error while executing smbclient: {0}")]
    ExecSmbClient(#[source] std::io::Error),
    #[error("error converting bytes to utf-8 stirng: {0}")]
    FromUtf8Error(#[from] std::string::FromUtf8Error),
    #[error("smbclient failed with {0}: {1}")]
    SmbClientFailed(ExitStatus, String),
    #[error("Unable to create socket pair for smbclient command: {0}")]
    SocketCreation(#[source] std::io::Error),
    #[error("Unable to write password over socket to smbclient: {0}")]
    WritePassword(#[source] std::io::Error),
    #[error("{0}")]
    Plugin(#[from] Box<Error>),
    #[error("{0}")]
    Keyvault(#[from] crate::config::KeyvaultError),
}
