/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;

use trust_dns_resolver::error::ResolveError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    KeyReader(agent_utils::Error),
    #[error("ETC Error: {0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("No credentials found in keyreader")]
    KeyReaderCredentials,
    #[error("Credentials not found in keyvault ({0})")]
    CredentialsNotFound(String),
    #[error("No credentials provided in config")]
    NoCredentialsProvided,
    #[error("Could not read file ({0}): ({1})")]
    IO(PathBuf, std::io::Error),
    #[error("Unable to deserialize json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Unable to execute command on server ({1}): {0}")]
    Command(async_ssh2_lite::Error, String),
    #[error("{0}")]
    Parser(String),
    #[error("{0}")]
    NoIPFound(ResolveError),
    #[error("Connection issue for port: {0} - {1}")]
    Connection(async_ssh2_lite::Error, u16),
    #[error("Failed to authenticate: {0}")]
    AuthenticationFailed(async_ssh2_lite::Error),
    #[error("Failed create an ssh session: {0}")]
    CreateSession(#[from] async_ssh2_lite::Error),
    #[error("SSH Session is not authenticated")]
    NotAuthenticated,
    #[error("No IP found for host {0}")]
    NoIP(String),
    #[error("Unable to decode string as utf-8: {0}")]
    Utf8(std::string::FromUtf8Error),
    #[error("Failed to find table_id in specfile: {0}")]
    Specfile(agent_utils::Error),
    #[error("{0}")]
    ShowQueries(std::fmt::Error),
}

pub type DTResult<T> = std::result::Result<T, DTError>;

#[derive(Debug, thiserror::Error)]
pub enum DTError {
    #[error("{0}")]
    Parser(String),
    #[error("Unable to execute command on server ({1}): {0}")]
    Command(#[source] async_ssh2_lite::Error, String),
    #[error("Unable to read result from command ({1}): {0}")]
    ReadChannel(#[source] std::io::Error, String),
    #[error("Failed to create subprocess for parser: {0} - {1}")]
    SubProcess(String, #[source] std::io::Error),
    #[error("Unable to deserialize json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Unable to decode string as utf-8: {0}")]
    Utf8(#[source] std::string::FromUtf8Error),
    #[error("Unable to create a channel to the server: {0}")]
    CreateChannel(#[source] async_ssh2_lite::Error),
    #[error("Could not retrieve exitstatus after executing command: {0}")]
    RetrieveExitStatus(#[source] async_ssh2_lite::Error),
    #[error("Command failed with exitstatus {0} and stderr: {1}")]
    CommandFailed(i32, String),
}

pub type DTWResult<T> = std::result::Result<T, DTWarning>;

#[derive(Debug, thiserror::Error)]
pub enum DTWarning {
    #[error("{0}")]
    Parser(String),
    #[error(
        "Tried to run command with sudo without 'allow_sudo' being enabled"
    )]
    SudoNotAllowed(),
    #[error("Failed to set env variable {0}: {1}")]
    SetEnvVariable(&'static str, #[source] async_ssh2_lite::Error),
}
