/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("ThruSSH: {0}")]
    ThruSSH(#[from] thrussh::Error),
    #[error("Invalid host argument: {0}")]
    Parse(String),
    /*#[error("Failed to resolve {0}: {1}")]
    ResolutionFailed(String, std::io::Error),
    #[error("Failed to resolve {0}: no results")]
    ResolutionEmpty(String),*/
}

impl<'a> From<nom::error::Error<&'a str>> for Error {
    fn from(val: nom::error::Error<&'a str>) -> Self {
        Error::Parse(val.to_string())
    }
}
