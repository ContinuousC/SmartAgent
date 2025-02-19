/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};

use etc_base::{ProtoDataFieldId, ProtoDataTableId};
use log::debug;
use protocol::auth;
use reqwest::Client;
use serde_json::Value;

use crate::{
    elastic::DTError,
    input::{FieldSpec, TableSpec},
};

use super::DTEResult;

pub struct DataTable<'a> {
    pub id: &'a ProtoDataTableId,
    pub spec: &'a TableSpec,
    pub fields: Arc<HashMap<&'a ProtoDataFieldId, &'a FieldSpec>>,
}

#[derive(Debug)]
pub struct Request<'a> {
    pub auth: &'a auth::BasicAuth,
    pub client: Client,
    pub base_url: &'a str,
    pub endpoint: &'a str,
    // pub stats: Vec<&'a str>
}

impl<'a> Request<'a> {
    fn format_url(&self) -> String {
        let params = {
            let mut ps = vec!["format=json"];
            if self.endpoint.starts_with("_cat") {
                ps.extend_from_slice(&["bytes=b", "time=ms"]);
            }
            ps.join("&")
        };

        // From experimentation, it turns out that its more performant
        //   to request everything instead of just whay we need.
        // I suspect elastic generates the entire document and filters after the fact
        // match self.endpoint {
        //     "_nodes/stats" =>  {
        //         let stats = self.stats.join(",");
        //         format!("{}/{}/{}?{}", self.base_url, self.endpoint, stats, params)
        //     }
        //     _ => format!("{}/{}?{}", self.base_url, self.endpoint, params)
        // }
        format!("{}/{}?{}", self.base_url, self.endpoint, params)
    }

    pub async fn call(&self) -> DTEResult<Value> {
        let url = self.format_url();
        debug!("requesting url: {url}");

        self.client
            .get(url)
            .basic_auth(&self.auth.username, self.auth.password.as_deref())
            .send()
            .await
            .map_err(DTError::SendRequest)?
            .error_for_status()
            .map_err(DTError::InvalidResponse)?
            .json()
            .await
            .map_err(DTError::InvalidResponse)
    }
}
