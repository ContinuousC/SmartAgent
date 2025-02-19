/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{error::Error as _, path::PathBuf};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("error retrieving proxmox credentials{0}")]
    RetrieveAuth(#[from] protocol::auth::Error),
    #[error("error creating proxmox client: {0}")]
    CreateClient(#[from] protocol::http::Error),
    #[error("ETC error: {0}")]
    AgentUtils(#[from] agent_utils::Error),

    #[error(
        "error sending request: {0} {}", 
        .0.cause().map(|c| format!("(cause: {c})")).unwrap_or_default()
    )]
    SendRequest(#[source] reqwest::Error),
    #[error(
        "Could not log in. Received an {} {}", 
        .0.status().unwrap(),
        .0.cause().map(|c| format!("(cause: {c})")).unwrap_or_default()
    )]
    FailedLogin(#[source] reqwest::Error),
    #[error(
        "cannot deserialize response: {0} {}", 
        .0.cause().map(|c| format!("(cause: {c})")).unwrap_or_default()
    )]
    DeserializeResponse(#[source] reqwest::Error),
}

pub type DTEResult<T> = std::result::Result<T, DTError>;

#[derive(Debug, thiserror::Error)]
pub enum DTError {
    #[error(
        "error sending request: {0} {}", 
        .0.cause().map(|c| format!("(cause: {c})")).unwrap_or_default()
    )]
    SendRequest(#[source] reqwest::Error),
    #[error(
        "invalid response: {0} {}", 
        .0.cause().map(|c| format!("(cause: {c})")).unwrap_or_default()
    )]
    InvalidResponse(#[source] reqwest::Error),
    #[error(
        "cannot deserialize response: {0} {}", 
        .0.cause().map(|c| format!("(cause: {c})")).unwrap_or_default()
    )]
    DeserializeResponse(#[source] reqwest::Error),
    #[error("invalid vmtype defined in resource ({0}). only {{qemuid}} and {{lxcid}} are allowed")]
    InvalidVmType(String),
    #[error("cannot access file {1}: {0}")]
    FileAccess(#[source] std::io::Error, PathBuf),

    #[error("{0}")]
    Custom(String),
}

pub type DTWResult<T> = std::result::Result<T, DTWarning>;

#[derive(Debug, thiserror::Error)]
pub enum DTWarning {}
