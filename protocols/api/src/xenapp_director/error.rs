/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

pub type Result<T> = std::result::Result<T, Error>;
pub type DTEResult<T> = std::result::Result<T, DTError>;
pub type DTWResult<T> = std::result::Result<T, DTWarning>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("{0}")]
    Auth(#[from] protocol::auth::Error),
    #[error("error creating client: {0}")]
    CreateClient(#[from] protocol::http::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum DTError {
    #[error("{0}")]
    Ntlm(#[from] protocol::auth::reqwest::NtlmError),
    #[error("failed to deserialize data: {0}")]
    QuickXml(#[from] quick_xml::DeError),
    #[error("unknown command")]
    UknownCommand,
    #[error("property was not expanded")]
    PropertyNotExpanded,
}

#[derive(Debug, thiserror::Error)]
pub enum DTWarning {}
