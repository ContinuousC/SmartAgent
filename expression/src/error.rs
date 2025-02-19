/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::convert::From;

use serde::Serialize;
use thiserror::Error;

use unit::UnitError;
use value::{DataError, Value};

pub type EvalResult = std::result::Result<Value, EvalError>;

#[derive(Error, Debug, Serialize, Clone)]
pub enum EvalError {
    #[error("Integer overflow")]
    IntegerOverflow,
    #[error("Divide by zero")]
    ZeroDivision,
    #[error("Index out of bounds")]
    OutOfBounds,
    #[error("Parse error: {0}")]
    NumParseError(&'static str),
    #[error("Parse error: {0}")]
    AddrParseError(&'static str),
    #[error("Missing variable: {0}")]
    MissingVariable(String),
    #[error("Error in referenced variable: {0}")]
    VariableError(String, Box<EvalError>),
    #[error("Recursion error")]
    RecursionError,
    #[error("Type error: {0}")]
    TypeError(&'static str),
    #[error("Value error: {0}")]
    ValueError(&'static str),
    #[error("Unit error: {0}")]
    UnitError(UnitError),
    #[error("Data error: {0}")]
    DataError(DataError),
    #[error("Expression parse error: {0}")]
    ParseError(String),
    #[error("Invalid format string")]
    FormatError(String),
    #[error("Invalid value (user-defined)")]
    InvalidValue,
    #[error("Error value (user-defined): {0}")]
    ErrorValue(String),
    #[error("Invalid utf8 data: {0}")]
    FromUtf8(String),
    // #[error("Invalid utf16 data: {0}")]
    // FromUtf16(String),
    #[error("Overflow in time / age calculation")]
    TimeOverflow,
    #[error("Error while using a selector: {0}")]
    Selector(String),
}

impl From<UnitError> for EvalError {
    fn from(err: UnitError) -> Self {
        Self::UnitError(err)
    }
}

impl From<DataError> for EvalError {
    fn from(err: DataError) -> Self {
        Self::DataError(err)
    }
}

impl<'f> From<dynfmt::Error<'f>> for EvalError {
    fn from(err: dynfmt::Error<'f>) -> Self {
        Self::FormatError(format!("{}", err))
    }
}

impl EvalError {
    pub fn is_missing_data(&self) -> bool {
        match self {
            Self::DataError(DataError::Missing)
            | Self::ErrorValue(_)
            | Self::InvalidValue => true,
            Self::VariableError(_, e) => e.is_missing_data(),
            _ => false,
        }
    }
}
