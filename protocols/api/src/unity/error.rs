/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use reqwest::StatusCode;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Entry not found in keyvault")]
    MissingKREntry,
    #[error("No {0} in KeyVault entry")]
    MissingKRObject(String),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("{0}")]
    Auth(#[from] protocol::auth::Error),

    #[error("failed to send request to {}: {}", .0.url().unwrap(), .0)]
    SendRequest(#[source] reqwest::Error),
    #[error("failed to recieve response {}: {}", .0.url().unwrap(), .0)]
    RecieveResponse(#[source] reqwest::Error),
    #[error("request to {0} failed {1}: {2}")]
    FailedRequest(String, StatusCode, String),
    #[error("failed to deserialize json: {0}")]
    DeserializeResponse(#[source] serde_json::Error),

    #[error("error creating client for unity: {0}")]
    CreateClient(#[from] protocol::http::Error),
    #[error("failed to log in: {}", .0.status().unwrap())]
    FailedLogin(#[source] reqwest::Error),
    #[error("failed to log out: {}", .0.status().unwrap())]
    FailedLogout(#[source] reqwest::Error),

    #[error("{0}")]
    Custom(String),
}

pub type DTEResult<T> = std::result::Result<T, DTError>;

#[derive(Debug, thiserror::Error)]
pub enum DTError {
    #[error("failed to send request to {}: {}", .0.url().unwrap(), .0)]
    SendRequest(#[source] reqwest::Error),
    #[error("failed to recieve response {}: {}", .0.url().unwrap(), .0)]
    RecieveResponse(#[source] reqwest::Error),
    #[error("request to {0} failed {1}: {2}")]
    FailedRequest(String, StatusCode, String),
    #[error("failed to deserialize json: {0}")]
    DeserializeResponse(#[source] serde_json::Error),
    #[error("metric {0} is not known on the unity device")]
    UknownMetric(String),
    #[error("the get_historic_metric command required the value field to be enabled")]
    ValuespecRequired,
    #[error("all textbased metrics are invalid")]
    TextBasedMetric,

    #[error("{0}")]
    Custom(String),
}

pub type DTWResult<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum DTWarning {}
