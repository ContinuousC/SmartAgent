/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{convert, fmt};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Authentication failed")]
    Authentication,
    #[error("Agent listener failed: {0}")]
    AgentListener(std::io::Error),
    #[error("Agent connect failed: {0}")]
    AgentConnect(std::io::Error),
    #[error("Backend listener failed: {0}")]
    BackendListener(std::io::Error),
    #[error("Database listener failed: {0}")]
    DatabaseListener(std::io::Error),
    #[error("Backend not connected")]
    BackendNotConnected,
    #[error("Broker channel closed unexpectedly!")]
    BrokerChannelClosed,
    #[error("Agent channel closed unexpectedly!")]
    AgentChannelClosed,
    #[error("Backend channel closed unexpectedly!")]
    BackendChannelClosed,
    #[error("Database channel closed unexpectedly!")]
    DatabaseChannelClosed,
    #[error("Failed reading from agent: {0}")]
    AgentStream(rpc::Error),
    //#[error("Failed reading from backend: {0}")]
    //BackendStream(api::Error),
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
    #[error("RPC error: {0}")]
    Rpc(#[from] rpc::Error),
    #[error("CBOR error: {0}")]
    Cbor(#[from] serde_cbor::Error),
    #[error("MPSC channel send error: {0}")]
    Tokio(Box<dyn std::error::Error + Send + Sync + 'static>),
    // #[error("TLS error: {0}")]
    // Tls(#[from] tokio_rustls::rustls::TLSError),
    #[error("Failed to load SSH key: {0}")]
    KeyDecode(thrussh_keys::Error),
    #[error("Failed to parse SSH host argument \"{0}\": {1}")]
    SshHostArg(String, ssh::Error),
    #[error("Ssh connection for {0} failed: {1}")]
    SshConnect(String, ssh::Error),
    #[error("Ssh authentication failed for {0}: {1}")]
    SshAuthenticate(String, thrussh::Error),
    #[error("Access for {0} was denied by the ssh server")]
    SshAuthentication(String),
    #[error("Failed to open TCP channel for {0}: {1}")]
    SshChannel(String, thrussh::Error),
    #[error("Failed to join SSH connector: {0}")]
    SshConnector(tokio::task::JoinError),
    #[error("Failed to install signal handler: {0}")]
    SignalInit(std::io::Error),
    #[error("Failed to send termination signal")]
    SendTerm,
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

impl From<Error> for broker_api::BrokerError {
    fn from(err: Error) -> Self {
        Self {
            retry: match err {
                Error::KeyDecode(_)
                | Error::SshHostArg(_, _)
                | Error::SshAuthentication(_) => false,
                _ => true,
            },
            message: err.to_string(),
        }
    }
}
