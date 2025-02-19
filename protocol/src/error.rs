/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use etc_base::{DataTableId, Protocol};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Invalid input for {0}: {1}")]
    InputFormat(Protocol, serde_path_to_error::Error<serde_json::Error>),
    #[error("Invalid config for {0}: {1}")]
    ConfigFormat(Protocol, serde_json::Error),
    #[error("Unknown input reference for {0}")]
    WrongInput(Protocol),
    #[error("Missing {0} protocol plugin")]
    MissingPlugin(Protocol),
    #[error("Missing input for {0}")]
    MissingInput(Protocol),
    #[error("Missing config for {0}")]
    MissingConfig(Protocol),
    #[error("{0} error: {1}")]
    Plugin(Protocol, Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error("remote plugin error: {0}: {1}")]
    RemotePlugin(Protocol, String),
    #[error("remote plugin initialization error: {0}")]
    RemotePluginInit(String),
}

#[derive(Serialize, Deserialize, Error, Debug)]
pub struct DataTableError {
    pub origin: ErrorOrigin,
    #[source]
    #[serde(with = "agent_serde::dyn_error")]
    pub error: Box<dyn std::error::Error + Send + Sync + 'static>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ErrorOrigin {
    Protocol(Protocol),
    DataTable(DataTableId),
}

impl fmt::Display for DataTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.origin {
            ErrorOrigin::Protocol(proto) => {
                write!(f, "{}: {}", proto, self.error)
            }
            ErrorOrigin::DataTable(id) => write!(f, "{}: {}", id, self.error),
        }
    }
}

/* Is there a way to separate this into the OMD module? */

/// Error category for dependency check.
#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum ErrorCategory {
    DataTable(DataTableId),
    Protocol(Protocol),
    Query,
    ETC,
    Agent,
}

impl ErrorCategory {
    /// "Data Table" and "Protocol" to use in dependency check.
    pub fn to_data_table_and_protocol(&self) -> Result<(String, String)> {
        match self {
            ErrorCategory::DataTable(id) => {
                Ok((id.1 .0.to_string(), id.0 .0.to_string()))
            }
            ErrorCategory::Protocol(id) => Ok((id.to_string(), id.to_string())),
            ErrorCategory::Agent => {
                Ok(("Agent".to_string(), "General".to_string()))
            }
            ErrorCategory::Query => {
                Ok(("Query".to_string(), "General".to_string()))
            }
            ErrorCategory::ETC => {
                Ok(("ETC".to_string(), "General".to_string()))
            }
        }
    }
}

impl DataTableError {
    pub fn omd_category(&self) -> ErrorCategory {
        match &self.origin {
            ErrorOrigin::Protocol(prot) => {
                ErrorCategory::Protocol(prot.clone())
            }
            ErrorOrigin::DataTable(id) => ErrorCategory::DataTable(id.clone()),
        }
    }
    pub fn omd_message(&self) -> String {
        self.error.to_string()
    }
}
