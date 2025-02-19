/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;
use std::{convert, env, error, fmt, io, net, result};

use thiserror::Error;

use etc_base::{DataTableId, Protocol};

/// Result type for use everywhere in the super agent
pub type Result<T> = result::Result<T, Error>;

/// Any agent error
#[derive(Error, Debug)]
pub enum Error {
    /* General */
    #[error("Unknown host: {0}")]
    UnknownHost(String),
    #[error("Parser error: {0}")]
    Parser(String),
    #[error("Invalid argument for {0}: {1}")]
    InvalidArgument(&'static str, String),
    #[error("Missing {0} (protocol plugin error)")]
    MissingDataTable(DataTableId),
    #[error("Missing {0} protocol plugin")]
    MissingPlugin(Protocol),
    #[error("Error retrieving plugin from pluginmanager: {0}")]
    PluginManager(#[from] protocol::Error),
    #[error("Error loading etc specs: {0}")]
    Etc(#[from] etc::Error),
    #[error("{0}")]
    Utils(#[from] agent_utils::Error),

    #[error("Data error: {0}")]
    DataError(#[from] value::DataError),
    #[error("Eval error: {0}")]
    EvalError(#[from] expression::EvalError),
    #[error("Unit error: {0}")]
    UnitError(#[from] unit::UnitError),
    #[error("Protocol error: {0}")]
    Protocol(#[from] protocol::DataTableError),
    #[error("Query error: {0}")]
    QueryError(#[from] query::QueryError),
    #[error("Failed parsing PEM file {0}")]
    InvalidPemFile(PathBuf),
    #[error("Missing environment variable \"{1}\" in file \"{0}\"")]
    MissingEnvVarInFile(PathBuf, String),

    /* Environment */
    #[error("I/O error: {0}")]
    IO(#[from] io::Error),
    #[error("Glob pattern error: {0}")]
    GlobPattern(#[from] glob::PatternError),
    #[error("Glob error: {0}")]
    GlobGlob(#[from] glob::GlobError),
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] env::VarError),
    #[error("JSON error: {0}")]
    JSON(#[from] serde_json::Error),
    #[error("CBOR error: {0}")]
    CBOR(#[from] serde_cbor::Error),
    #[error("Argument error: {0}")]
    Clap(#[from] clap::Error),
    #[error("Tokio error: {0}")]
    Tokio(#[from] Box<dyn error::Error + Sync + Send + 'static>),
    #[error("TLS error: {0}")]
    TLSError(#[from] rustls::Error),
    #[error("Invalid address: {0}")]
    InvalidAddr(#[from] net::AddrParseError),
    #[error("Invalid dns name: {0}")]
    InvalidDnsName(#[from] webpki::InvalidDnsNameError),
    #[error("The name of the Specfile is invalid: {0}")]
    InvalidSpecFileName(PathBuf),

    /* Password vault */
    #[error("Failed to run key-reader: {0}")]
    KeyReader(#[source] io::Error),
    #[error("Password vault entry not found!")]
    MissingPWEntry,
    #[error("Password vault entry has no password!")]
    MissingPassword,

    /* OMD */
    #[error("Missing field in error file: {0}")]
    MissingDependencyField(&'static str),
    #[error("Invalid value for field {0} in error file: {1}")]
    InvalidDependencyField(&'static str, String),
    #[error("This is not an OMD site!")]
    NotOMD,
    // #[error("OMD config script error: {0}")]
    // OMDConfigScript(&'static str),

    /* Protocols */
    #[error("SNMP: {0}")]
    SNMP(#[from] snmp_protocol::Error),
    #[error("Azure: {0}")]
    AZURE(#[from] azure_protocol::AzureError),
    #[error("Azure: {0}")]
    WMI(#[from] wmi_protocol::WMIError),
    #[error("API: {0}")]
    API(#[from] api_protocol::Error),

    /* Custom */
    #[error("{0}")]
    Custom(String),
}

impl<T> convert::From<tokio::sync::mpsc::error::SendError<T>> for Error
where
    T: fmt::Debug + Send + Sync + 'static,
{
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Error {
        Error::Tokio(Box::new(err))
    }
}

impl convert::From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(err: tokio::sync::oneshot::error::RecvError) -> Error {
        Error::Tokio(Box::new(err))
    }
}
