/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::error::Error as _;

use protocol::{auth, http};

use serde_json::Value as JsonValue;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("unable to create http client: {0}")]
    CreateClient(#[from] http::Error),
    #[error("unable to find credentials: {0}")]
    CredentialLookup(#[from] auth::Error),
}

pub type PathResult<T> = std::result::Result<T, PathError>;

#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error(
        "the next step in the path could not be found: {0} (remaining: {1})"
    )]
    StepNotFound(String, String),
    #[error("encouted an invalid type when pasing the path. expected a {1}, found {0}")]
    InvalidType(JsonValue, &'static str),
}

pub type DTEResult<T> = std::result::Result<T, DTError>;

#[derive(Debug, thiserror::Error)]
pub enum DTError {
    #[error(
        "error sending request: {0} {}", 
        .0.cause().map(|c| format!("(cause: {c})"))
            .unwrap_or_default()
    )]
    SendRequest(#[source] reqwest::Error),
    #[error(
        "recieved an invalid response ({}) {}",
        .0.status().unwrap(),
        .0.cause().map(|c| format!("(cause: {c})"))
            .unwrap_or_default()
    )]
    InvalidResponse(#[source] reqwest::Error),
    #[error("unable to deserialize response: {0}")]
    DeserializeResponse(#[source] reqwest::Error),
    #[error("the inner table was not found in the response from elastic")]
    NotFound,
    #[error("the table that this command points to is invalid")]
    InvalidTable,
    #[error("could not receive path form value: {0}")]
    PathError(#[from] PathError),

    #[error("{0}")]
    Custom(String),
}

pub type DTWResult<T> = std::result::Result<T, DTWarning>;

#[derive(Debug, thiserror::Error)]
pub enum DTWarning {}
