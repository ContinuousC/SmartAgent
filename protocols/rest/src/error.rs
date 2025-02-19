/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use reqwest::header::InvalidHeaderValue;
use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RESTError {
    #[error("Unable to fill in template: {0} (original body: {1})")]
    ParseToken(#[source] TemplateError, String),
    #[error("Error filling in template: {0}")]
    TemplateError(#[from] TemplateError),
    #[error("Unable to serialize/deserialize data: {0}")]
    SerdeJSONError(#[from] serde_json::Error),
    #[error("Unable to encode url: {0}")]
    SerdeUrlEncodedError(#[from] serde_urlencoded::ser::Error),
    #[error("Error during HTTP request: {0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Unable to compult jsonschema: {0}")]
    CompilationError(String),
    #[error("Error in response (validated with jsonschema): {0:?}")]
    ValidationError(Vec<String>),
    #[error("Tried sending a request with an invalid header: {0}")]
    InvalidHeaderValue(#[from] InvalidHeaderValue),
}

#[derive(Error, Debug)]
pub enum TemplateError {
    #[error("Error during parsing")]
    ParseError,
    #[error("Missing variable: {0}")]
    MissingVariable(String),
    #[error("Value {0} ({1}) is not a string")]
    NotAString(String, Value),
}
