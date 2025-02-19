/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use async_recursion::async_recursion;
use async_trait::async_trait;
use futures::{stream, StreamExt};
use jsonpath::Selector;
use log::{info, trace, warn};
use reqwest::{Client, Response, StatusCode};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use tokio::time::{sleep, Duration};

use agent_utils::{KeyVault, TryGetFrom};
use etc_base::{Annotated, ProtoDataFieldId, ProtoDataTableId, ProtoQueryMap};
use value::DataError;

use super::error::{DTEResult, DTError, Result};
use crate::error::Result as APIResult;
use crate::input::{FieldSpec, PluginId, TableSpec};
use crate::ms_graph::definitions::LicenseSku;
use crate::ms_graph::parsers::{deserialize_csv, parse_jsonval, parse_val};
use crate::ms_graph::ResourceResponse;
use crate::plugin::TableData;
use crate::{ms_graph::Config, plugin::DataMap, Input};
use crate::{APIPlugin, Plugin as ProtPlugin};

pub static MSGRAPH_ENDPOINT: &str = "https://graph.microsoft.com/v1.0";

pub struct Plugin {
    key_vault: KeyVault,
    pub config: Config,
}

#[async_recursion]
pub async fn request_with_retry(
    client: &Client,
    url: &str,
    retries: u16,
) -> DTEResult<Response> {
    trace!("requesting {url} with {retries} attempts");
    if retries == 0 {
        warn!("no retries left for {}", &url);
        Err(DTError::ToManyRetries(url.to_string()))
    } else {
        match client.get(url).send().await {
            Err(e) => Err(DTError::ReqwestError(e)),
            Ok(r) => {
                if r.status() == StatusCode::TOO_MANY_REQUESTS {
                    info!("request for '{}' failed due to throtteling. {} retries left", url, retries);
                    sleep(Duration::from_secs(1)).await;
                    request_with_retry(client, url, retries - 1).await
                } else {
                    Ok(r)
                }
            }
        }
    }
}

#[derive(Deserialize)]
struct ServiceResponse {
    pub value: Vec<JsonValue>,
}

#[derive(Deserialize)]
struct MappedServiceResponse {
    pub value: Vec<HashMap<String, JsonValue>>,
}

#[derive(Deserialize)]
struct GroupsResponse {
    pub value: Vec<Group>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Group {
    pub id: String,
    pub resource_provisioning_options: Vec<String>,
    pub display_name: String,
}

impl Plugin {
    pub fn new(key_vault: KeyVault, config: Config) -> Result<Self> {
        Ok(Self { key_vault, config })
    }

    async fn exec_query(
        &self,
        client: &Client,
        dt_id: ProtoDataTableId,
        command: TableSpec,
        fields: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> (ProtoDataTableId, TableData) {
        let (endpoint, table_args) = match command.command_line.split_once('|')
        {
            Some(tup) => tup,
            None => (command.command_line.as_str(), ""),
        };
        let url = format!("{}/{}", MSGRAPH_ENDPOINT, endpoint);
        info!("retrieving datatable: {:?} ({})", &dt_id, &url);
        let response = match request_with_retry(client, &url, 3).await {
            Err(e) => return (dt_id, Err(e.to_api())),
            Ok(r) => r,
        };
        let status = response.status();
        info!("{:?} returned status {}", &dt_id, &status);

        (
            dt_id.clone(),
            match response.text().await {
                Err(e) => Err(DTError::ReqwestError(e).to_api()),
                Ok(response) => {
                    if status == StatusCode::FORBIDDEN {
                        warn!("{:?} forbidden request", &dt_id);
                        Err(DTError::Forbidden(url).to_api())
                    } else {
                        match command.command_name.as_str() {
                            "get_state" => self.get_state(
                                &command.command_line,
                                response,
                                fields,
                            ),
                            "get_rapport" => self.get_rapport(
                                &command.command_line,
                                response,
                                fields,
                            ),
                            "get_internal_table_with_root" => self
                                .get_internal_table_with_rootid(
                                    &command.command_line,
                                    table_args,
                                    response,
                                    fields,
                                ),
                            "get_channels" => {
                                self.get_channels(client, response, fields)
                                    .await
                            }
                            "get_licenceskus" => {
                                self.get_license_skus(client, response, fields)
                                    .await
                            }
                            s => Err(DTError::CommandNotFound(s.to_string())
                                .to_api()),
                        }
                    }
                }
            },
        )
    }

    async fn get_channels(
        &self,
        client: &Client,
        request: String,
        fields: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> TableData {
        let groups: GroupsResponse =
            serde_json::from_str(&request).map_err(DTError::SerdeJsonError)?;
        let teams = groups
            .value
            .into_iter()
            .filter(|g| {
                g.resource_provisioning_options
                    .contains(&String::from("Team"))
            })
            .collect::<Vec<Group>>();

        let mut channels: Vec<JsonValue> = Vec::new();
        for team in teams {
            let response = request_with_retry(
                client,
                &format!("{}/teams/{}/channels", MSGRAPH_ENDPOINT, team.id),
                10,
            )
            .await?
            .text()
            .await
            .map_err(|e| DTError::ReqwestError(e).to_api())?;
            let data: MappedServiceResponse =
                serde_json::from_str(&response)
                    .map_err(DTError::SerdeJsonError)?;
            channels.reserve(data.value.len());

            for mut channel in data.value {
                channel.insert(
                    String::from("teamId"),
                    JsonValue::String(team.id.clone()),
                );
                channel.insert(
                    String::from("teamDisplayName"),
                    JsonValue::String(team.display_name.clone()),
                );
                channels.push(
                    serde_json::to_value(channel)
                        .map_err(DTError::SerdeJsonError)?,
                )
            }
        }

        self.get_from_json(&String::new(), channels, fields)
    }

    async fn get_license_skus(
        &self,
        client: &Client,
        response: String,
        fields: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> TableData {
        let mut skus: ResourceResponse<LicenseSku> =
            serde_json::from_str(&response).map_err(DTError::SerdeJsonError)?;

        let reverence = {
            const LICENSE_PLAN_REFERENCE: &str = "https://download.microsoft.com/download/e/3/e/e3e9faf2-f28b-490a-9ada-c6089a1fc5b0/Product%20names%20and%20service%20plan%20identifiers%20for%20licensing.csv";
            let response =
                request_with_retry(client, LICENSE_PLAN_REFERENCE, 3).await?;
            let response =
                response.text().await.map_err(DTError::ReqwestError)?;
            deserialize_csv(response)?
        };

        for sku in skus.value.iter_mut() {
            let id = sku.sku_id.to_string();
            let pretty_name = reverence
                .iter()
                .find_map(|rf| {
                    matches!(rf.get("GUID"), Some(guid) if guid == &id)
                        .then(|| rf.get("Product_Display_Name").cloned())
                })
                .flatten();

            if let Some(pn) = pretty_name {
                sku.sku_part_number = pn;
            }
        }

        let skus = skus
            .value
            .into_iter()
            .map(|sku| serde_json::to_value(sku).unwrap())
            .collect();

        self.get_from_json(&String::new(), skus, fields)
    }

    fn get_internal_table_with_rootid(
        &self,
        cmd_line: &String,
        table_args: &str,
        request: String,
        fields: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> TableData {
        let (table_path, root_id) = table_args.split_once('|').ok_or(
            DTError::EtcSyntaxError(String::from("Invalid Table Arguments")),
        )?;
        let table_selector = Selector::new(table_path).map_err(|e| {
            DTError::JsonPathError(table_path.to_string(), e.to_string())
        })?;
        let root_id_selector = Selector::new(root_id).map_err(|e| {
            DTError::JsonPathError(root_id.to_string(), e.to_string())
        })?;
        let data: ServiceResponse =
            serde_json::from_str(&request).map_err(DTError::SerdeJsonError)?;

        let internal_table = data
            .value
            .into_iter()
            .map(|root_obj| {
                table_selector
                    .find(&root_obj)
                    .map(|json_obj| -> DTEResult<JsonValue> {
                        let mut obj = json_obj
                            .as_object()
                            .ok_or(DTError::ParseJsonObject(
                                json_obj.clone(),
                                String::from("object"),
                            ))?
                            .clone();
                        match root_id_selector.find(&root_obj).next() {
                            Some(id) => {
                                obj.insert(String::from("ROOTID"), id.clone());
                            }
                            None => warn!(
                                "root_id_selector {} not found in {}",
                                root_id, &root_obj
                            ),
                        }
                        Ok(JsonValue::Object(obj))
                    })
                    .collect::<DTEResult<Vec<JsonValue>>>()
            })
            .filter_map(|res| match res {
                Ok(v) => Some(v),
                Err(e) => {
                    warn!("{}", e);
                    None
                }
            })
            .flatten()
            .collect::<Vec<JsonValue>>();

        self.get_from_json(cmd_line, internal_table, fields)
    }

    fn get_state(
        &self,
        cmd_line: &String,
        request: String,
        fields: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> TableData {
        let data: ServiceResponse =
            serde_json::from_str(&request).map_err(DTError::SerdeJsonError)?;

        self.get_from_json(cmd_line, data.value, fields)
    }

    fn get_from_json(
        &self,
        cmd_line: &String,
        data: Vec<JsonValue>,
        fields: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> TableData {
        let selectors = fields
            .iter()
            .map(|(df_id, field)| {
                Ok((
                    df_id.clone(),
                    Selector::new(&field.parameter_header).map_err(|e| {
                        DTError::JsonPathError(
                            field.parameter_header.clone(),
                            e.to_string(),
                        )
                    })?,
                ))
            })
            .collect::<DTEResult<HashMap<ProtoDataFieldId, Selector>>>()?;
        let mut rows = Vec::new();
        let is_mesages = cmd_line == "admin/serviceAnnouncement/messages";

        for value in data {
            // added filter for messages. we are not interested in messages past their end date
            if is_mesages && self.filter_message(&value).unwrap_or(false) {
                continue;
            }

            let mut row = HashMap::with_capacity(fields.len());
            for (df_id, field) in fields.iter() {
                let mut results = selectors
                    .get(df_id)
                    .unwrap()
                    .find(&value)
                    .collect::<Vec<&JsonValue>>();
                row.insert(
                    df_id.clone(),
                    if results.is_empty() {
                        Err(DataError::Missing)
                    } else {
                        parse_jsonval(field, results.pop().unwrap().clone())
                            .map_err(|e| DataError::TypeError(e.to_string()))
                    },
                );
            }
            rows.push(row);
        }

        Ok(Annotated {
            value: rows,
            warnings: Vec::new(),
        })
    }

    fn get_rapport(
        &self,
        cmd_line: &str,
        request: String,
        fields: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> TableData {
        let mut rows = Vec::new();
        let mut data = deserialize_csv(request)?
            .into_iter()
            .filter(|row| {
                !row.get("Is Deleted")
                    .unwrap_or(&String::from("False"))
                    .to_lowercase()
                    .parse::<bool>()
                    .unwrap_or(false)
            })
            .collect::<Vec<HashMap<String, String>>>();

        // sorting & filter for requests with a large output
        data = if data.is_empty() {
            Vec::new()
        } else if data.first().unwrap().contains_key("Report Date") {
            vec![data.into_iter().next().unwrap()]
        } else {
            self.filter_rapport(cmd_line, data)
        };

        for data_row in data {
            rows.push(
                fields
                    .iter()
                    .map(|(df_id, field)| {
                        (
                            df_id.clone(),
                            if let Some(val) =
                                data_row.get(&field.parameter_header)
                            {
                                parse_val(field, val).map_err(|e| {
                                    DataError::TypeError(e.to_string())
                                })
                            } else {
                                Err(DataError::Missing)
                            },
                        )
                    })
                    .collect(),
            );
        }

        Ok(Annotated {
            value: rows,
            warnings: Vec::new(),
        })
    }
}

#[async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        info!("Using MS Graph plugin");

        let client = self.config.login(&self.key_vault).await?;

        let mut requests = Vec::new();
        for (dt_id, df_ids) in query {
            let command = ProtPlugin::get_datatable_id(dt_id)
                .try_get_from(&input.data_tables)?;
            if command.plugin != PluginId(String::from("ms_graph")) {
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
