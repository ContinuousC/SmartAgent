/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::dimension::Dimension;
use super::unit::Unit;

#[derive(Serialize, Deserialize, Error, PartialEq, Eq, Clone, Debug)]
pub enum UnitError {
    #[error("Unsupported unit operation: {0} * {1}")]
    Mul(Dimension, Dimension),
    #[error("Unsupported unit operation: {0} / {1}")]
    Div(Dimension, Dimension),
    #[error("Unsupported unit operation: {0} ^ {1}")]
    Pow(Dimension, i32),
    #[error("Unsupported unit composition: {0} * {1}")]
    CMul(Unit, Unit),
    #[error("Unsupported unit composition: {0} / {1}")]
    CDiv(Unit, Unit),
    #[error("Unsupported unit composition: {0} ^ {1}")]
    CPow(Unit, i32),
    #[error("Incompatible units: {0} <-> {1}")]
    Conversion(Dimension, Dimension),
    #[error("Unit parse error: {0}")]
    ParseError(String),
    #[error("invalid unit {1} for dimension {0}")]
    TypeError(Dimension, Unit),
    #[error("unsupported dimension: {0}")]
    Unsupported(Dimension),
    #[error("JSON error: {0}")]
    Json(String),
}
