/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use super::template::Template;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum BodyType {
    JSON,
    FormUrlEncoded,
    None, // FormData
}

impl BodyType {
    pub fn mime_type(&self) -> String {
        match self {
            Self::FormUrlEncoded => {
                String::from("application/x-www-form-urlencoded")
            }
            Self::JSON => String::from("application/json"),
            Self::None => String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum ContentType {
    JSON, //XML
}

impl ContentType {
    pub fn mime_type(&self) -> String {
        match self {
            Self::JSON => String::from("application/json"),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum AuthType {
    Token(Template),
    Cookie,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum HTTPMethod {
    GET,
    POST,
}
