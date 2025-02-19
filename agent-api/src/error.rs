/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Peer requested unsupported protocol version")]
    VersionMismatch,
    #[error("Received unexpected message from peer")]
    Protocol,
    #[error("Backend not connected")]
    BackendNotConnected,
    #[error("DbDaemon not connected")]
    DatabaseNotConnected,
    #[error("Authentication failed")]
    AuthenticationFailed,
    #[error("Received error from peer: {0}")]
    Peer(String),
    #[error("Unsupported API function")]
    Unsupported,
    #[error("The message is too long")]
    MessageTooLong,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON en-/decoding error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("CBOR en-/decoding error: {0}")]
    Cbor(#[from] serde_cbor::Error),
    #[error("Invalid message in CBOR stream: {0:?}")]
    InvalidCborMessage(serde_cbor::Value),
    #[error("Stream reached EOF")]
    Eof,
}
