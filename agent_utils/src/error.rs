/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Incompatible definitions for {0}")]
    IncompatibleDefinitions(String),
    #[error("Missing {0}")]
    MissingObject(String),

    #[cfg(feature = "trust-dns-resolver")]
    #[error("failed to resolve hostname: {0}")]
    Resolve(trust_dns_resolver::error::ResolveError),
    #[cfg(feature = "trust-dns-resolver")]
    #[error("failed to resolve hostname: {0}")]
    ResolveIo(std::io::Error),
    #[cfg(feature = "trust-dns-resolver")]
    #[error("hostname resolution did not yield an ip address")]
    ResolveMissing,

    /* Password vault */
    #[cfg(feature = "key-reader")]
    #[error("Failed to run key-reader: {0}")]
    KeyReader(#[from] std::io::Error),
    #[cfg(feature = "key-reader")]
    #[error("Key-reader stream error: {0}")]
    AuthStream(#[from] key_reader::stream::Error),
    #[error("Password vault entry not found!")]
    MissingPWEntry,
    #[error("Password vault entry has no password!")]
    MissingPassword,
}
