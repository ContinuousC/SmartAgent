/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
    #[error("unexpected nmap output: {0}")]
    XML(#[from] serde_xml_rs::Error),
    #[error("Failed to parse {0} output: {1}")]
    Parse(&'static str, String),
    #[error("nmap exited with code {0:?}: {1}")]
    NonZeroExitStatus(Option<i32>, String),
}

pub(crate) type ParseBytesResult<T, E> =
    std::result::Result<T, ParseBytesError<E>>;

#[derive(Error, Debug)]
pub(crate) enum ParseBytesError<E: std::error::Error> {
    #[error("Utf8 error: {0}")]
    Utf8(std::str::Utf8Error),
    #[error("Parse error: {0}")]
    Parse(E),
}
