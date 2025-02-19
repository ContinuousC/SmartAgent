/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;
use std::{convert, env, error, fmt, io, net, result};

use thiserror::Error;

use etc_base::DataTableId;

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
    #[error("RPC error: {0}")]
    Rpc(#[from] rpc::Error),
    #[error("Failed parsing PEM file {0}")]
    InvalidPemFile(PathBuf),
    #[error("Missing environment variable \"{1}\" in file \"{0}\"")]
    MissingEnvVarInFile(PathBuf, String),

    /* Environment */
    #[error("I/O error: {0}")]
    IO(#[from] io::Error),
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

    /* OMD */
    #[error("This is not an OMD site!")]
    NotOMD,
    #[error("OMD config script error: {0}")]
    OMDConfigScript(&'static str),

    /* Custom */
    #[error("{0}")]
    Custom(String),

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

impl convert::From<tokio::sync::oneshot::error::RecvError> for Error {
    fn from(err: tokio::sync::oneshot::error::RecvError) -> Error {
        Error::Tokio(Box::new(err))
    }
}
