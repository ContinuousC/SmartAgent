/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;

use super::from_xml::XmlInput;

pub type ParseResult<'a, T> =
    std::result::Result<(T, XmlInput<'a>), ParseError>;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("{0}")]
    Syntax(&'static str),
    #[error("unexpected tag: {0}")]
    UnexpectedTag(String),
    #[error("unexpected end tag: {0}")]
    UnexpectedEndTag(String),
    #[error("unexpected input: {0:?}")]
    Unexpected(xml::reader::XmlEvent),
    #[error("unexpected end of input")]
    Eof,
}
