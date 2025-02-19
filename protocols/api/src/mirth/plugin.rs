/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use agent_utils::{KeyVault, TryGetFrom};
use etc_base::{Annotated, ProtoDataFieldId, ProtoQueryMap, ProtoRow, Warning};
use futures::{stream, StreamExt};
use log::{debug, error, info, trace, warn};
use logger::Verbosity;
use protocol::CounterDb;
use tap::TapFallible;
use uuid::Uuid;
use value::{DataError, Value};

use super::responses::{
    ChannelConnector, ChannelGroups, ChannelStatistics, ChannelStatuss,
    ConnectorType,
};
use super::{api, smb, Config, DTEResult, DTWResult, Error, Result};
use crate::error::Result as APIResult;
use crate::input::{FieldSpec, TableSpec};
use crate::mirth::responses::{ChannelSpecific, SystemInfo};
use crate::mirth::DTError;
use crate::plugin::TableData;
use crate::{plugin::DataMap, APIPlugin, Input, Plugin as ProtPlugin};

pub struct Plugin {
    key_vault: KeyVault,
    cache_dir: PathBuf,
    config: Config,
}

struct Request<'a> {
    datatable: &'a TableSpec,
    datafields: HashMap<&'a ProtoDataFieldId, &'a FieldSpec>,
    api: Arc<api::Client>,
    smb: Arc<Option<smb::Client<'a>>>,
    counter_db: Arc<CounterDb>,
}

impl Plugin {
    pub fn new(
        cache_dir: PathBuf,
        key_vault: KeyVault,
        config: Config,
    ) -> Self {
        Self {
            key_vault,
            cache_dir,
            config,
        }
    }

    pub async fn init_api_client(&self) -> Result<Arc<api::Client>> {
        let client = api::Client::new(
            &self.config.http_client,
            &self.config.api_auth,
            self.key_vault.clone(),
        )
        .await?;
        client.login().await?;
        Ok(Arc::new(client))
    }

    pub async fn init_smb_client(&self) -> Result<Arc<Option<smb::Client>>> {
        let client = match (&self.config.smb_auth, &self.config.smb_opts) {
            (Some(auth), Some(opts)) => {
                Some(smb::Client::new(auth, opts, &self.key_vault).await?)
            }
            _ => None,
        };

        //TODO: some form of login/verification?
        Ok(Arc::new(client))
    }

    async fn get_channelgroups(
        &self,
        api: Arc<api::Client>,
    ) -> DTEResult<ChannelGroups> {
        api.get_endpoint("channelgroups")
            .await
            .map_err(DTError::Api)
            .tap_err(|e| error!("failed to retrieve channelgroups: {e}"))
    }

    async fn request_datatable<'a>(&self, req: Request<'a>) -> TableData {
        // channel_status: gname, cid, name, state
        // channel_stats: gname, cid, rx, tx, err, filt, queue
        // connectors: gname, cid, path, scheme, count, type
        // sys_info: jvm, os, db

        match req.datatable.command_line.as_str() {
            "channel_groups" => self.request_channel_groups(req).await,
            "channel_status" => self.request_channel_status(req).await,
            "channel_stats" => self.request_channel_stats(req).await,
            "connectors" => self.request_connectors(req).await,
            "sys_info" => self.request_system_info(req).await,
            _ => Err(crate::error::DTError::CommandNotFound(
                req.datatable.command_line.clone(),
            )),
        }
    }

    async fn request_channel_groups<'a>(&self, req: Request<'a>) -> TableData {
        let channelgroups = self.get_channelgroups(req.api.clone()).await?;
        trace!("retrieved channelgroups: {channelgroups:#?}");

        Ok(Annotated {
            value: channelgroups.get_data(req.datafields),
            warnings: Vec::new(),
        })
    }

    async fn request_channel_status<'a>(&self, req: Request<'a>) -> TableData {
        let stati: ChannelStatuss = req
            .api
            .get_endpoint("channels/statuses")
            .await
            .map_err(DTError::Api)
            .tap_err(|e| error!("failed to retrieve channel statuses: {e}"))?;
        trace!("retrieved channel statuses: {stati:#?}");

        let rows = stati
            .data
            .into_iter()
            .map(|ch| {
                req.datafields
                    .iter()
                    .map(|(id, f)| ((*id).clone(), ch.get_data(f)))
                    .collect()
            })
            .collect();

        Ok(Annotated {
            value: rows,
            warnings: Vec::new(),
        })
    }

    async fn request_channel_stats<'a>(&self, req: Request<'a>) -> TableData {
        let stati: ChannelStatistics = req
            .api
            .get_endpoint("channels/statistics")
            .await
            .map_err(DTError::Api)
            .tap_err(|e| {
                error!("failed to retrieve channel statistics: {e}")
            })?;
        trace!("retrieved channel statistics: {stati:#?}");

        let rows = stati
            .data
            .into_iter()
            .map(|ch| {
                req.datafields
                    .iter()
                    .map(|(id, field)| {
                        (
                            (*id).clone(),
                            ch.get_data(field, req.counter_db.clone()),
                        )
                    })
                    .collect()
            })
            .collect();

        Ok(Annotated {
            value: rows,
            warnings: Vec::new(),
        })
    }

    async fn request_connector<'a>(
        &self,
        id: Uuid,
        req: Arc<Request<'a>>,
        connector: ChannelConnector,
        ctype: ConnectorType,
    ) -> Option<DTWResult<ProtoRow>> {
        // nothing to check
        if connector.properties.scheme.is_none() {
            return None;
        }

        let mut row = HashMap::new();
        for (fid, f) in &req.datafields {
            // connectors: gname, cid, path, scheme, count, type
            row.insert(
                (*fid).clone(),
                match f.parameter_header.as_str() {
                    "channel_id" => Ok(Value::UnicodeString(id.to_string())),
                    "connector_name" => connector.name.to_smartm_value(),
                    "scheme" => connector.properties.scheme.to_smartm_value(),
                    "path" => connector.properties.host.to_smartm_value(),
                    "type" => ctype.to_smartm_value(f),
                    "count" => match (
                        &connector.properties.host.data,
                        &connector.properties.scheme.data
                    ) {
                        (Some(host), Some(scheme)) => {
                            match scheme.as_ref() {
                                "FILE" => {
                                    if let Some(smb) = req.smb.as_ref() {
                                        debug!("sending smb request to host: {host}");
                                        smb.listdir(host.as_str()).await
                                            .tap_err(|e| warn!("failed to monitor smb connector: {e}"))
                                            .map_err(|e| DataError::External(format!("failed to request smb files: {e}")))
                                            .map(|files| Value::Integer(files.len() as i64))
                                    } else {
                                        Err(DataError::External("smb config not set".to_string()))
                                    }
                                },
                                _ => Err(DataError::External(format!("unsupported scheme: {scheme}")))
                            }
                        },
                        _ => Err(DataError::External("host or scheme are not set".to_string()))
                    }

                    _ => Err(DataError::Missing)
                }
            );
        }

        Some(Ok(row))
    }

    async fn request_connectors_for_channel<'a>(
        &self,
        id: Uuid,
        req: Arc<Request<'a>>,
    ) -> DTWResult<Vec<DTWResult<ProtoRow>>> {
        let channel: ChannelSpecific = req
            .api
            .get_endpoint(format!("channels/{id}"))
            .await
            .map_err(DTError::Api)
            .tap_err(|e| error!("failed to retrieve channel statuses: {e}"))?;
        trace!("retrieved channel info: {channel:#?}");

        let requests = channel
            .destination_connectors
            .data
            .into_iter()
            .map(|conn| {
                self.request_connector(
                    id,
                    req.clone(),
                    conn,
                    ConnectorType::Destination,
                )
            })
            .chain([self.request_connector(
                id,
                req.clone(),
                channel.source_connector,
                ConnectorType::Source,
            )])
            .collect::<Vec<_>>();

        let num_requests = requests.len();
        debug!(
            "scheduled {num_requests} requests to monitor channel connectors"
        );

        let results = stream::iter(requests)
            .buffer_unordered(num_requests)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect();

        Ok(results)
    }

    async fn request_connectors<'a>(&self, req: Request<'a>) -> TableData {
        let req = Arc::new(req);
        let channelgroups = self.get_channelgroups(req.api.clone()).await?;

        let requests = channelgroups
            .get_channels()
            .into_iter()
            .map(|id| self.request_connectors_for_channel(id, req.clone()));

        let num_requests = requests.len();
        debug!("scheduled {num_requests} requests to monitor channels",);
        let results: Vec<_> = stream::iter(requests)
            .buffer_unordered(num_requests)
            .collect()
            .await;

        let mut rows = Vec::with_capacity(results.len());
        let mut warns = Vec::with_capacity(results.len());

        for result in results {
            match result {
                Err(e) => warns.push(e),
                Ok(r) => {
                    for result in r {
                        match result {
                            Err(e) => warns.push(e),
                            Ok(r) => rows.push(r),
                        }
                    }
                }
            }
        }

        Ok(Annotated {
            value: rows,
            warnings: warns
                .into_iter()
                .map(|w| Warning {
                    message: crate::error::DTWarning::Mirth(w),
                    verbosity: Verbosity::Warning,
                })
                .collect(),
        })
    }

    async fn request_system_info<'a>(&self, req: Request<'a>) -> TableData {
        let sysinfo: SystemInfo = req
            .api
            .get_endpoint("system/info")
            .await
            .map_err(DTError::Api)?;

        let row = req
            .datafields
            .into_iter()
            .map(|(id, f)| (id.clone(), sysinfo.get_data(f)))
            .collect::<HashMap<_, _>>();

        Ok(Annotated {
            value: vec![row],
            warnings: Vec::new(),
        })
    }
}

#[async_trait::async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        info!("using mirth plugin");
        let api = self.init_api_client().await?;
        info!("api client succesfully initialized");
        let smb = self.init_smb_client().await?;
        let counters = CounterDb::load(self.cache_dir.join("counters.json"))
            .await
            .map(Arc::new)?;
        debug!("loaded counters: {counters:#?}");

        let mut futures = Vec::with_capacity(query.len());
        for (dt_id, df_ids) in query {
            let dt = ProtPlugin::get_datatable_id(dt_id)
                .try_get_from(&input.data_tables)?;
            let dfs = df_ids
                .iter()
                .map(|df_id| {
                    Ok((
                        df_id,
                        ProtPlugin::get_datafield_id(df_id)
                            .try_get_from(&input.data_fields)
                            .map_err(Error::AgentUtils)?,
                    ))
                })
                .collect::<Result<HashMap<_, _>>>()?;

            let request = Request {
                datatable: dt,
                datafields: dfs,
                api: api.clone(),
                smb: smb.clone(),
                counter_db: counters.clone(),
            };

            info!("scheduled request for {}", &request.datatable.command_name);
            futures.push(async {
                (dt_id.clone(), self.request_datatable(request).await)
            })
        }

        let result = stream::iter(futures)
            .buffer_unordered(query.len())
            .collect::<DataMap>()
            .await;

        counters.save().await?;

        Ok(result)
    }
}
