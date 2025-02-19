/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use async_trait::async_trait;
use futures::{stream, StreamExt};
use jsonpath::Selector;
use log::{debug, info};
use reqwest::Client;
use serde_json::Value;

use agent_utils::{KeyVault, TryGetFrom};
use azure_protocol::Config;
use etc_base::{Annotated, ProtoQueryMap};
use etc_base::{ProtoDataFieldId, ProtoDataTableId, ProtoRow};
use rest_protocol::{http::HTTPMethod, input::RESTRequest, Template};
use uritemplate::UriTemplate;

use crate::azure::{DTEResult, DTError, Response, Result};
use crate::error::Result as APIResult;
use crate::input::{FieldSpec, PluginId, TableSpec};
use crate::plugin::{DataMap, TableData};
use crate::{APIPlugin, Input, Plugin as ProtPlugin};

pub struct Plugin {
    key_vault: KeyVault,
    pub config: Config,
}

impl Plugin {
    pub fn new(key_vault: KeyVault, config: Config) -> Result<Self> {
        Ok(Self { key_vault, config })
    }

    fn create_request(&self) -> DTEResult<RESTRequest> {
        Ok(RESTRequest {
            url: UriTemplate::new(
                "https://management.azure.com/subscriptions/{subscription}/providers/{provider}/{resource}?api-version={version}"
            ),
            data: vec![
                    (String::from("subscription"), Template::parse("{{subscription}}")?),
                    (String::from("provider"), Template::parse("{{provider}}")?),
                    (String::from("resource"), Template::parse("{{resource}}")?),
                    (String::from("version"), Template::parse("{{version}}")?)
                ].into_iter().collect(),
            method: HTTPMethod::GET,
            schema: Value::Null,
            reference: None
        })
    }

    async fn exec_request(
        &self,
        client: &Client,
        // dt_id: ProtoDataTableId,
        command: TableSpec,
        fields: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> TableData {
        debug!("requesting table: {:?}", &command);
        let selectors = fields.iter()
            .map(|(df_id, field)| {
                Ok((
                    df_id.clone(),
                    (field.clone(), Selector::new(&field.parameter_header).map_err(|e| {
                        DTError::JsonPathError(
                            field.parameter_header.clone(),
                            e.to_string(),
                        )
                    })?),
                ))
            })
            .collect::<DTEResult<HashMap<ProtoDataFieldId, (FieldSpec, Selector)>>>()
            .map_err(|e| e.to_api())?;
        let mut input: HashMap<String, String> = vec![
            (String::from("provider"), command.command_line.clone()),
            (String::from("resource"), command.command_name.clone()),
            (String::from("version"), command.command_description.clone()),
        ]
        .into_iter()
        .collect();

        let empty = Vec::new();
        let subscriptions =
            self.config.subscriptions.as_ref().unwrap_or(&empty);
        let mut requests = Vec::with_capacity(subscriptions.len());
        for subscription in subscriptions {
            input.insert(String::from("subscription"), subscription.clone());
            requests.push(self.request_subscription(
                input.clone(),
                &selectors,
                client,
            ));
        }

        let responses: Vec<_> = stream::iter(requests)
            .buffer_unordered(subscriptions.len())
            .collect()
            .await;
        let mut data = Vec::new();
        for response in responses.into_iter() {
            match response {
                Err(e) => return Err(crate::error::DTError::Azure(e)),
                Ok(d) => data.extend(d),
            }
        }

        Ok(Annotated {
            value: data,
            warnings: Vec::new(),
        })
    }

    async fn request_subscription(
        &self,
        input: HashMap<String, String>,
        selectors: &HashMap<ProtoDataFieldId, (FieldSpec, Selector)>,
        client: &Client,
    ) -> DTEResult<Vec<ProtoRow>> {
        let mut request = self.create_request()?;
        let response: Response = serde_json::from_str(
            &request
                .execute(client, &input)
                .await
                .map_err(DTError::RESTError)?,
        )
        .map_err(DTError::SerdeJsonError)?;
        Ok(response.to_datatable(selectors))
    }

    async fn exec_query(
        &self,
        client: &Client,
        dt_id: ProtoDataTableId,
        command: TableSpec,
        fields: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> (ProtoDataTableId, TableData) {
        (
            dt_id.clone(),
            self.exec_request(client, command, fields).await,
        )
    }
}

#[async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        info!("Using Azure API plugin");

        let client = self
            .config
            .login(Some(&self.key_vault))
            .await
            .map_err(super::error::Error::Azure)?;
        debug!("Logged in successfully");

        let mut requests = Vec::new();
        for (dt_id, df_ids) in query {
            let command = ProtPlugin::get_datatable_id(dt_id)
                .try_get_from(&input.data_tables)?;
            if command.plugin != PluginId(String::from("azure")) {
                continue; // shouldn't happend
            }
            let fields = df_ids
                .iter()
                .map(|df_id| {
                    Ok((
                        df_id.clone(),
                        ProtPlugin::get_datafield_id(df_id)
                            .try_get_from(&input.data_fields)?
                            .clone(),
                    ))
                })
                .collect::<Result<HashMap<ProtoDataFieldId, FieldSpec>>>()?;
            info!(
                "planning to request {:?} with {} fields",
                &dt_id,
                fields.len()
            );
            requests.push(self.exec_query(
                &client,
                dt_id.clone(),
                command.clone(),
                fields,
            ));
        }

        info!("planned {} requests", requests.len());
        let data = stream::iter(requests).buffer_unordered(8).collect().await;
        info!("all requests completed");

        Ok(data)
    }
}
