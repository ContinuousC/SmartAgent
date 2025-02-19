/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

#[cfg(feature = "tokio")]
use std::sync::Arc;

use thiserror::Error;

#[cfg(feature = "tokio")]
use super::spec::Spec;
use etc_base::PackageName;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid package data in {0}: {1}")]
    PackageData(PackageName, serde_json::Error),
    #[error("{0}")]
    Utils(#[from] agent_utils::Error),
    #[error("Protocol error: {0}")]
    Protocol(#[from] protocol::Error),
    #[cfg(feature = "tokio")]
    #[error("Failed to distribute new etc definitions: {0}")]
    SendSpec(#[from] tokio::sync::watch::error::SendError<Arc<Spec>>),
}
