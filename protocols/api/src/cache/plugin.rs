/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::sync::Mutex;
use std::{collections::HashMap, fmt::Debug, path::PathBuf, sync::Arc};

use agent_utils::{KeyVault, TryGetFrom};
use async_trait::async_trait;
use etc_base::{Annotated, ProtoDataFieldId, ProtoDataTableId, ProtoQueryMap};

use futures::{stream, StreamExt};
use protocol::CounterDb;
use reqwest::Client;
use serde::de::DeserializeOwned;
use tap::pipe::Pipe;

use crate::error::DTError as DTErrorAPI;
use value::Data;

use crate::cache::types::{
    BodyDashboard, BodyECPAppSvr, BodyECPDataSvr, BodyEnumBuffer,
    BodyEnumDatabase, BodyEnumProcess, BodyEnumResource, BodyEnumWriteDaemon,
    BodyGlobal, BodyRoutine, BodySystem,
};

use crate::{
    cache::DTError,
    error::Result as APIResult,
    input::{FieldSpec, TableSpec},
    plugin::{DataMap, TableData},
    APIPlugin, Input, Plugin as ProtPlugin,
};

use super::types::generic::CreateTabledata;
use super::{error::Error, types::ApiResponse, Config, DTResult};

// these are the same for all xml messages
const XML_PART1: &str = "<?xml version='1.0' encoding='utf-8'?><soap-env:Envelope xmlns:soap-env='http://schemas.xmlsoap.org/soap/envelope/'><soap-env:Body><ns0:";
const XML_PART2: &str = " xmlns:ns0='http://www.intersystems.com/cache/wsmon/1'/></soap-env:Body></soap-env:Envelope>";

pub struct Plugin {
    keyvault: KeyVault,
    config: Config,
    counter_database: PathBuf,
}

impl Plugin {
    pub fn new(
        keyvault: KeyVault,
        config: Config,
        counter_database: PathBuf,
    ) -> Plugin {
        Plugin {
            keyvault,
            config,
            counter_database,
        }
    }

    async fn post(
        &self,
        url: &str,
        table: &TableSpec,
        client: &Client,
    ) -> APIResult<String> {
        let full_xml =
            format!("{}{}{}", XML_PART1, table.command_line, XML_PART2);

        let response = client
            .post(url)
            .header(
                "SOAPAction",
                format!("http://www.intersystems.com/cache/wsmon/1/SYS.WSMon.Service.{}",table.command_line),
            )
            .header("Content-Type", "text/xml")
            .header("charset", "utf-8")
            .body(full_xml)
            .send()
            .await.map_err(Error::Reqwest)?;

        Ok(response.text().await.map_err(Error::Reqwest)?)
    }

    fn parse_xml<T: DeserializeOwned + Debug + Send + CreateTabledata>(
        &self,
        xml: &str,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<Mutex<CounterDb>>,
    ) -> DTResult<Vec<HashMap<ProtoDataFieldId, Data>>> {
        let response: DTResult<ApiResponse<T>> = quick_xml::de::from_str(xml)
            .map_err(|e| DTError::ParseXml(e, xml.to_string()));

        response.map(|y| y.body.create_tabledata(fields, counterdb))
    }

    fn parse_xml_to_hashmap(
        &self,
        table: &TableSpec,
        response_body: &str,
        counter_db: Arc<Mutex<CounterDb>>,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
    ) -> DTResult<Vec<HashMap<ProtoDataFieldId, Data>>> {
        Ok(match table.command_line.as_str() {
            "GetSystem" => {
                self.parse_xml::<BodySystem>(response_body, fields, counter_db)?
            }
            "GetRoutine" => self.parse_xml::<BodyRoutine>(
                response_body,
                fields,
                counter_db,
            )?,
            "GetGlobal" => {
                self.parse_xml::<BodyGlobal>(response_body, fields, counter_db)?
            }
            "GetECPDataSvr" => self.parse_xml::<BodyECPDataSvr>(
                response_body,
                fields,
                counter_db,
            )?,
            "GetECPAppSvr" => self.parse_xml::<BodyECPAppSvr>(
                response_body,
                fields,
                counter_db,
            )?,
            "GetDashboard" => self.parse_xml::<BodyDashboard>(
                response_body,
                fields,
                counter_db,
            )?,
            "EnumBuffer" => self.parse_xml::<BodyEnumBuffer>(
                response_body,
                fields,
                counter_db,
            )?,
            "EnumDatabase" => self.parse_xml::<BodyEnumDatabase>(
                response_body,
                fields,
                counter_db,
            )?,
            "EnumProcess" => self.parse_xml::<BodyEnumProcess>(
                response_body,
                fields,
                counter_db,
            )?,
            "EnumWriteDaemon" => self.parse_xml::<BodyEnumWriteDaemon>(
                response_body,
                fields,
                counter_db,
            )?,
            "EnumResource" => self.parse_xml::<BodyEnumResource>(
                response_body,
                fields,
                counter_db,
            )?,
            _ => {
                return Err(DTError::UnknownCommand(table.command_name.clone()))
            }
        })
    }

    async fn get_data(
        &self,
        table: &TableSpec,
        table_id: ProtoDataTableId,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        client: &Client,
        counter_db: &Arc<Mutex<CounterDb>>,
        hostname: String,
    ) -> (ProtoDataTableId, TableData) {
        let url = format!(
            "https://{}:{}/csp/sys/SYS.WSMon.Service.cls",
            hostname,
            self.config.port.unwrap_or(443)
        );

        let mut response = match self.post(&url, table, client).await {
            Ok(value) => value,
            Err(e) => {
                return (
                    table_id,
                    Err(e).map_err(|p| DTErrorAPI::Cache(DTError::Post(p))),
                )
            }
        };

        let mut parsed_data = self.parse_xml_to_hashmap(
            table,
            &response,
            counter_db.clone(),
            fields.clone(),
        );

        if parsed_data.is_err() {
            // If parsing doesn't work, login again to set new cookies and try again.
            match self.config.login(client, &self.keyvault).await {
                Ok(_) => (),
                Err(e) => {
                    return (
                        table_id,
                        Err(e)
                            .map_err(|p| DTErrorAPI::Cache(DTError::Cache(p))),
                    )
                }
            }
            response = match self.post(&url, table, client).await {
                Ok(value) => value,
                Err(e) => {
                    return (
                        table_id,
                        Err(e).map_err(|p| DTErrorAPI::Cache(DTError::Post(p))),
                    )
                }
            };
            parsed_data = self.parse_xml_to_hashmap(
                table,
                &response,
                counter_db.clone(),
                fields.clone(),
            );
        }

        let formatted_data = parsed_data
            .map(|tabledata| {
                Ok(Annotated {
                    value: tabledata,
                    warnings: Vec::with_capacity(0),
                })
            })
            .map_err(crate::error::DTError::Cache);

        (table_id, formatted_data.and_then(std::convert::identity))
    }
}

#[async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        log::info!("Start running Caché plugin");
        let counter_db =
            CounterDb::load(self.counter_database.join("counters.json"))
                .await
                .unwrap_or_else(|_| {
                    CounterDb::new(self.counter_database.clone())
                })
                .pipe(Mutex::new)
                .pipe(Arc::new);

        let client = self.config.init_client().await?;

        self.config.login(&client, &self.keyvault).await?;

        let mut requests = Vec::new();
        for (table_id, field_ids) in query {
            let table = ProtPlugin::get_datatable_id(table_id)
                .try_get_from(&input.data_tables)?;

            let fields = field_ids
                .clone()
                .iter()
                .map(|field_id| {
                    ProtPlugin::get_datafield_id(field_id)
                        .try_get_from(&input.data_fields)
                        .map(|field| (field_id.clone(), field))
                })
                .collect::<Result<
                    HashMap<ProtoDataFieldId, &FieldSpec>,
                    agent_utils::Error,
                >>()?;
            let hostname = self.config.get_hostname().await?;
            requests.push(self.get_data(
                table,
                table_id.clone(),
                fields,
                &client,
                &counter_db,
                hostname.clone(),
            ));
        }
        log::info!("Caché requests prepared, start fetching data...");

        let data = stream::iter(requests)
            .buffer_unordered(8)
            .collect::<HashMap<ProtoDataTableId, TableData>>()
            .await;

        log::info!("Caché data collected!");

        Arc::into_inner(counter_db)
            .unwrap()
            .into_inner()
            .unwrap()
            .save()
            .await
            .map_err(Error::CounterDbSave)?;

        log::info!("Counter DB saved!");

        Ok(data)
    }
}
