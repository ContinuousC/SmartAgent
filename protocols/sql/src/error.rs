/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use etc_base::DataFieldId;
use trust_dns_resolver::error::ResolveError;

use crate::config::InstanceType;

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Could not find field in spec: {0}")]
    FieldNotFound(DataFieldId),
    #[error("hostname {0} could not be resolved: {1}")]
    Dns(String, #[source] ResolveError),
    #[error("No Ip found for {0}")]
    IpNotFound(String),
    #[error("Unable to browse server for mssql instances: {0}")]
    MsSqlBrowse(String),
    #[error("invalid instance for sql plugin {1}: {0}")]
    InvalidInstance(InstanceType, &'static str),
    #[error("Error Querying ETC Database: {0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("Error formatting query: {0}")]
    Format(#[from] std::fmt::Error),
    #[error("Cannot connect to database instance {0}: {1}")]
    Connection(InstanceType, #[source] odbc_api::Error),
    #[error("Cannot retrieve the databases from instance {0}: {1}")]
    DatabaseQuery(InstanceType, #[source] Box<DTError>),
    #[error("Cannot create a counter database: {0}")]
    CounterDbCreation(#[source] std::io::Error),
    #[error("Cannot save the counter database to disk: {0}")]
    CounterDbSave(#[source] std::io::Error),
    #[error("No valuetype set for enum")]
    NoValueType,
    #[error("Querying instances timed out ({0}s)")]
    Timeout(u64),
    #[error("{0}")]
    Custom(String),
}

pub type DTEResult<T> = std::result::Result<T, DTError>;
#[derive(Debug, thiserror::Error)]
pub enum DTError {
    #[error("Query failed to execute: {0}")]
    FailedQuery(#[source] odbc_api::Error),
    #[error("Query returned an empty result")]
    EmptyResult,
    #[error(
        "Database query returned no database cullumn containing databases"
    )]
    NoDatabaseColumn,
    #[error("Unable to retrieve metadata from query statement: {0}")]
    Metadata(#[source] odbc_api::Error),
    #[error("Unabel to bind a buffer to a query statement")]
    BufferBind(#[source] odbc_api::Error),
    #[error("Unable to fetch a row from the query statement")]
    FetchRow(#[source] odbc_api::Error),
    #[error("Could not construct the desired query: {0}")]
    ConstructQuery(#[source] Box<Error>),
    #[error("Could not found field in table: {0}")]
    FieldNotFound(&'static str),
    #[error("Could not parse the provided value to an integer: {0}")]
    ParseInteger(#[from] std::num::ParseIntError),
    #[error("Could not deserialize countertype")]
    DeserializeCounter(#[from] ron::de::SpannedError),
}

pub type DTWResult<T> = std::result::Result<T, DTWarning>;
#[derive(Debug, thiserror::Error)]
pub enum DTWarning {
    #[error("Failed to query instance: {0}")]
    Instance(String),
    #[error("Field '{0}' not found in row")]
    FieldNotFound(String),
    #[error("{0}")]
    DTError(#[from] DTError),
    #[error("{0}")]
    Error(String),
}
