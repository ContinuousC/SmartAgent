/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::fmt::Debug;

use reqwest::{header::HeaderMap, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::RESTError;
use super::http::*;
use super::template::Template;
use uritemplate::UriTemplate;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Config {
    #[serde(rename = "Name")]
    pub(super) name: String,
    #[serde(rename = "Description")]
    pub(super) description: String,
    /*
    #[serde(rename = "Type")]
    pub(super) r#type: Type
    */
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Application {
    #[serde(rename = "ContentType")]
    pub content_type: ContentType,
    #[serde(rename = "AuthType")]
    pub auth_type: AuthType,
    #[serde(rename = "LoginURL")]
    pub login_url: UriTemplate,
    #[serde(rename = "LoginHTTPMethod")]
    pub login_method: HTTPMethod,
    #[serde(rename = "LoginData")]
    pub login_data: HashMap<String, Template>,
    #[serde(rename = "LoginBodyType")]
    pub login_body_type: BodyType,
    /*
    #[serde(rename = "RestConfigVar")]
    pub(super) rest_config_var: Vec<Config>,
    */
}

impl Application {
    pub async fn login(
        &mut self,
        creds: &HashMap<String, String>,
    ) -> Result<Client, RESTError> {
        let filledin_data = self
            .login_data
            .iter()
            .map(|(k, v)| v.fill_in(creds).map(|val| (k.clone(), val)))
            .collect::<Result<HashMap<String, String>, crate::TemplateError>>(
            )?;
        for (k, v) in creds {
            self.login_url.set(k, v.clone());
        }
        match self.login_method {
            HTTPMethod::POST => {
                let client = Client::builder().cookie_store(true).build()?;
                let request = client
                    .post(self.login_url.build())
                    .header("Content-Type", self.login_body_type.mime_type())
                    .header("Accept", self.content_type.mime_type())
                    .body(match self.login_body_type {
                        BodyType::JSON => {
                            serde_json::to_string(&filledin_data)?
                        }
                        BodyType::FormUrlEncoded => {
                            serde_urlencoded::to_string(&filledin_data)?
                        }
                        BodyType::None => String::new(),
                    })
                    .build()?;
                let response = client.execute(request).await?;

                match &self.auth_type {
                    AuthType::Cookie => Ok(client),
                    AuthType::Token(template) => {
                        let mut headers = HeaderMap::new();
                        let response_body =
                            &response.json::<HashMap<String, Value>>().await?;
                        headers.insert(
                            "Authorization",
                            template
                                .fill_in(
                                    &response_body
                                        .iter()
                                        .filter(|(_k, v)| v.is_string())
                                        .map(|(k, v)| {
                                            (
                                                k.to_string(),
                                                v.as_str().unwrap().to_string(),
                                            )
                                        })
                                        .collect::<HashMap<String, String>>(),
                                )
                                .map_err(|e| {
                                    RESTError::ParseToken(
                                        e,
                                        format!("{response_body:?}"),
                                    )
                                })?
                                .parse()?,
                        );
                        Ok(Client::builder()
                            .default_headers(headers)
                            .build()?)
                    }
                }
            }
            HTTPMethod::GET => {
                panic!("{:?}", "not supported")
            }
        }
    }
}
