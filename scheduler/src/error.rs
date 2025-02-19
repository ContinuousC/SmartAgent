/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{convert, error, fmt};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Tokio error: {0}")]
    Tokio(#[from] Box<dyn error::Error + Sync + Send + 'static>),
    #[error("Tokio error: {0}")]
    TokioJoin(#[from] tokio::task::JoinError),
    #[error("{0}")]
    Utils(#[from] agent_utils::Error),
    #[error("Protocol error: {0}")]
    Protocol(#[from] protocol::Error),
    #[error("Query error: {0}")]
    Query(#[from] query::QueryError),
    #[error("Etc error: {0}")]
    Etc(#[from] etc::Error),
    #[error("Failed to convert config to raw value: {0}")]
    ConfigToRaw(serde_json::Error),
    #[error("timeout")]
    Timeout,
}

impl<T> convert::From<tokio::sync::mpsc::error::SendError<T>> for Error
where
    T: fmt::Debug + Send + Sync + 'static,
{
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Error {
        Error::Tokio(Box::new(err))
    }
}

impl<T> convert::From<tokio::sync::watch::error::SendError<T>> for Error
where
    T: fmt::Debug + Send + Sync + 'static,
{
    fn from(err: tokio::sync::watch::error::SendError<T>) -> Error {
        Error::Tokio(Box::new(err))
    }
}
