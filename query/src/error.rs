/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::convert::From;
use std::fmt;
use std::sync::Arc;

use etc_base::{AnnotatedResult, Warning};
use etc_base::{DataFieldId, DataTableId};
use protocol::DataTableError;
use protocol::ErrorCategory;
use thiserror::Error;
use value::Type;

use super::key_set::KeySet;

pub type QueryResult<T> = std::result::Result<T, QueryError>;
pub type QueryCheckResult<T> = std::result::Result<T, QueryTypeError>;
pub type AnnotatedQueryResult<T> = AnnotatedResult<T, QueryWarning, QueryError>;

/// Runtime query errors.
#[derive(Error, Clone, Debug)]
pub enum QueryError {
    #[error("the table does not exist ({})", .0.iter().map(
	|Warning {message,..}| message.to_string()).collect::<Vec<_>>().join(", "))]
    DoesntExist(Vec<Warning<QueryWarning>>),
    #[error("{0}")]
    TypeError(QueryTypeError),
    #[error("{0}")]
    Protocol(Arc<DataTableError>),
    #[error("join lead to many x many cross")]
    Cross,
}

/// Query errors that can be detected at compile-time.
#[derive(thiserror::Error, Clone, Debug)]
pub enum QueryTypeError {
    #[error("Unhashable key {0} of type {1}")]
    UnhashableKey(DataFieldId, Type),
    #[error("Empty ETC-based query")]
    EmptyTableQuery,
    #[error("Type mismatch in prefilter on {0}: got {1}, expected {2}")]
    FilterTypeError(DataFieldId, Type, Type),
    #[error("Join key length mismatch")]
    JoinKeyLengthMismatch,
    #[error("Join key type mismatch: {0} ({3}) vs {1} ({2})")]
    JoinKeyTypeMismatch(DataFieldId, DataFieldId, Type, Type),
    #[error("Missing primary key in join: add either {0} to the left or {1} to the right")]
    NoPrimaryKey(KeySet, KeySet),
    #[error("Missing data table: {0}")]
    MissingDataTable(DataTableId),
    #[error("Missing field: {0}")]
    MissingField(DataFieldId),
    #[error("Missing key {0}")]
    MissingKey(DataFieldId),
}

#[derive(Clone, Debug)]
pub enum QueryWarning {
    DTWarning(Arc<DataTableError>),
    DTError(Arc<DataTableError>),
}

impl QueryError {
    /// Category to use in OMD Dependency Check: data_table, protocol
    pub fn omd_category(&self) -> ErrorCategory {
        match self {
            Self::Protocol(err) => err.omd_category(),
            Self::TypeError(_) => ErrorCategory::ETC,
            Self::Cross => ErrorCategory::ETC,
            Self::DoesntExist(_) => ErrorCategory::Agent,
        }
    }

    /// Message to show in OMD Dependency Check.
    pub fn omd_message(&self) -> String {
        match self {
            Self::Protocol(err) => err.omd_message(),
            _ => self.to_string(),
        }
    }
}

impl QueryWarning {
    /// Category to use in OMD Dependency Check: data_table, protocol
    pub fn omd_category(&self) -> ErrorCategory {
        match self {
            Self::DTError(err) => err.omd_category(),
            Self::DTWarning(err) => err.omd_category(),
        }
    }

    /// Message to show in OMD Dependency Check.
    pub fn omd_message(&self) -> String {
        match self {
            Self::DTError(err) => err.omd_message(),
            Self::DTWarning(err) => err.omd_message(),
        }
    }
}

impl fmt::Display for QueryWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DTError(err) => write!(f, "{}", err),
            Self::DTWarning(err) => write!(f, "{}", err),
        }
    }
}

impl From<QueryTypeError> for QueryError {
    fn from(err: QueryTypeError) -> Self {
        Self::TypeError(err)
    }
}
