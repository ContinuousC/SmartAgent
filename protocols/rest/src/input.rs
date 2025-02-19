/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use crate::RESTError;
use reqwest::{Client, Request};
use serde::{Deserialize, Serialize};
use uritemplate::UriTemplate;

use super::http::*;
use super::template::Template;
use log::{debug, info};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DataTable {
    #[serde(rename = "request")]
    pub(super) request: Box<RESTRequest>,
    #[serde(rename = "JQProgram")]
    pub(super) json_path: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct RESTRequest {
    #[serde(rename = "url")]
    pub url: UriTemplate,
    #[serde(rename = "data")]
    pub data: HashMap<String, Template>,
    #[serde(rename = "HTTPMethod")]
    pub method: HTTPMethod,
    #[serde(rename = "ResponseSchema")]
    pub schema: Value,
    #[serde(rename = "Reference")]
    pub reference: Option<Box<DataTable>>,
}

impl RESTRequest {
    pub async fn execute(
        &mut self,
        client: &Client,
        wato: &HashMap<String, String>,
    ) -> Result<String, RESTError> {
        let filledin_data = self
            .data
            .iter()
            .map(|(k, v)| (k.clone(), v.fill_in(wato).unwrap()))
            .collect::<HashMap<String, String>>();
        debug!("filledin_data: {:?}", &filledin_data);

        debug!("preparing url: {:?}", &self.url);
        for (k, v) in &filledin_data {
            debug!("setting variable: {}", &k);
            self.url.set(k, v.clone());
        }
        let url = &self.url.build();
        debug!("actual url: {}", &url);

        let request: Request = match self.method {
            HTTPMethod::GET => client.get(url.clone()).build()?,
            HTTPMethod::POST => panic!("{}", "Not yet implemented"),
        };

        let response = client.execute(request).await?;
        info!("request to {:?} returned {}", &url, response.status());
        let text = response.text().await?;
        debug!("with data: {}", &text);
        Ok(text)
    }
}
