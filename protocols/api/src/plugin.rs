/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::fmt::Write;
use std::path::PathBuf;

use agent_utils::{KeyVault, TryGetFrom};
use async_trait::async_trait;
use etc_base::{
    AnnotatedResult, DataFieldId, DataTableId, ProtoDataFieldId,
    ProtoDataTableId, ProtoQueryMap, ProtoRow, Protocol,
};
use futures::{stream, StreamExt};
use log::{debug, info, warn};

use crate::error::{DTError, DTWarning, Error, Result, TypeError, TypeResult};
use crate::input::PluginId;
use crate::{
    cache, elastic, ldap, mirth, ms_graph, proxmox, unity, vmware,
    xenapp_director, Config, Input,
};
use protocol::{DataFieldSpec, DataTableSpec, LocalPlugin};

pub type TableData = AnnotatedResult<Vec<ProtoRow>, DTWarning, DTError>;
pub type DataMap = HashMap<ProtoDataTableId, TableData>;

pub struct Plugin {
    key_vault: KeyVault,
    cache_dir: PathBuf,
}

impl Plugin {
    pub fn new(cache_dir: PathBuf, key_vault: KeyVault) -> Self {
        Self {
            key_vault,
            cache_dir,
        }
    }
    pub fn get_datatable_id(dt_id: &ProtoDataTableId) -> DataTableId {
        DataTableId(Protocol(Plugin::PROTOCOL.to_string()), dt_id.clone())
    }
    pub fn get_datafield_id(df_id: &ProtoDataFieldId) -> DataFieldId {
        DataFieldId(Protocol(Plugin::PROTOCOL.to_string()), df_id.clone())
    }
}

#[async_trait]
pub trait APIPlugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> Result<DataMap>;
}

#[async_trait]
impl LocalPlugin for Plugin {
    type Error = Error;
    type TypeError = TypeError;
    type DTError = DTError;
    type DTWarning = DTWarning;

    type Input = Input;
    type Config = Config;

    const PROTOCOL: &'static str = "API";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn show_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> Result<String> {
        let mut out = String::new();
        for (resource_id, field_ids) in query {
            let command = Self::get_datatable_id(resource_id)
                .try_get_from(&input.data_tables)?;
            writeln!(
                out,
                "API (plugin: {}): Command: {} ({}) with params: {}",
                command.plugin.0,
                command.command_name,
                command.command_line,
                field_ids
                    .iter()
                    .map(|df_id| Self::get_datafield_id(df_id)
                        .try_get_from(&input.data_fields))
                    .map(|param| match param {
                        Ok(param) => format!(
                            "{} (header: {}, type: {})",
                            param.parameter_name,
                            param.parameter_header,
                            param.parameter_type
                        ),
                        Err(e) => e.to_string(),
                    })
                    .collect::<Vec<String>>()
                    .join(", ")
            )?;
        }
        Ok(out)
    }

    fn get_tables(
        &self,
        input: &Self::Input,
    ) -> TypeResult<HashMap<ProtoDataTableId, DataTableSpec>> {
        Ok(input
            .data_tables
            .keys()
            .map(|dt_id| {
                let datafields = input
                    .data_table_fields
                    .get(dt_id)
                    .cloned()
                    .unwrap_or_default();
                (
                    dt_id.1.clone(),
                    DataTableSpec {
                        name: dt_id.1 .0.clone(),
                        singleton: false,
                        keys: datafields
                            .iter()
                            .map(|id| (id, input.data_fields.get(id)))
                            .filter_map(|(id, field)| {
                                if let Some(field) = field {
                                    match field.is_key {
                                        true => Some(id),
                                        false => None,
                                    }
                                } else {
                                    None
                                }
                            })
                            .map(|id| id.1.clone())
                            .collect(),
                        fields: datafields.into_iter().map(|id| id.1).collect(),
                    },
                )
            })
            .collect())
    }

    fn get_fields(
        &self,
        input: &Self::Input,
    ) -> TypeResult<HashMap<ProtoDataFieldId, DataFieldSpec>> {
        input
            .data_fields
            .iter()
            .map(|(df_id, field_spec)| {
                Ok((
                    df_id.1.clone(),
                    DataFieldSpec {
                        name: field_spec.parameter_name.clone(),
                        input_type: field_spec.get_type()?,
                    },
                ))
            })
            .collect::<TypeResult<HashMap<ProtoDataFieldId, DataFieldSpec>>>()
    }

    async fn run_queries(
        &self,
        input: &Input,
        config: &Config,
        query: &ProtoQueryMap,
    ) -> Result<DataMap> {
        let mut plugins: HashMap<PluginId, Box<dyn APIPlugin + Send>> =
            HashMap::new();

        if let Some(conf) = &config.vmware {
            plugins.insert(
                PluginId(String::from("vmware")),
                Box::new(vmware::Plugin::new(
                    self.cache_dir.clone(),
                    self.key_vault.clone(),
                    conf.clone(),
                )?),
            );
        }
        if let Some(conf) = &config.ms_graph {
            plugins.insert(
                PluginId(String::from("ms_graph")),
                Box::new(ms_graph::Plugin::new(
                    self.key_vault.clone(),
                    conf.clone(),
                )?),
            );
        }
        if let Some(conf) = &config.azure {
            plugins.insert(
                PluginId(String::from("azure")),
                Box::new(crate::azure::Plugin::new(
                    self.key_vault.clone(),
                    conf.clone(),
                )?),
            );
        }
        if let Some(conf) = &config.ldap {
            plugins.insert(
                PluginId(String::from("ldap")),
                Box::new(
                    ldap::Plugin::new(
                        self.key_vault.clone(),
                        conf.clone(),
                        self.cache_dir.join("ldap"),
                    )
                    .await?,
                ),
            );
        }
        if let Some(conf) = &config.cache {
            plugins.insert(
                PluginId(String::from("cache")),
                Box::new(cache::Plugin::new(
                    self.key_vault.clone(),
                    conf.clone(),
                    self.cache_dir.join("cache"),
                )),
            );
        }
        if let Some(conf) = &config.mirth {
            plugins.insert(
                PluginId(String::from("mirth")),
                Box::new(mirth::Plugin::new(
                    self.cache_dir.join("mirth"),
                    self.key_vault.clone(),
                    conf.clone(),
                )),
            );
        }

        if let Some(conf) = &config.unity {
            plugins.insert(
                PluginId(String::from("unity")),
                Box::new(unity::Plugin::new(
                    self.cache_dir.join("unity"),
                    self.key_vault.clone(),
                    conf.clone(),
                )),
            );
        }

        if let Some(conf) = &config.xenapp_director {
            plugins.insert(
                PluginId(String::from("xenapp_director")),
                Box::new(xenapp_director::Plugin::new(
                    self.key_vault.clone(),
                    conf.clone(),
                )),
            );
        }

        if let Some(conf) = &config.proxmox {
            plugins.insert(
                PluginId(String::from("proxmox")),
                Box::new(proxmox::Plugin::new(
                    self.cache_dir.join("proxmox"),
                    self.key_vault.clone(),
                    conf.clone(),
                )),
            );
        }

        if let Some(conf) = &config.elastic {
            plugins.insert(
                PluginId(String::from("elastic")),
                Box::new(elastic::Plugin::new(
                    self.cache_dir.join("elastic"),
                    self.key_vault.clone(),
                    conf.clone(),
                )),
            );
        }
        debug!(
            "registerd plugins: {:?}",
            plugins.keys().collect::<HashSet<_>>()
        );

        let mut plugin_requests: HashMap<PluginId, ProtoQueryMap> =
            HashMap::with_capacity(plugins.len());
        for (dt_id, df_ids) in query {
            let cmd = Self::get_datatable_id(dt_id)
                .try_get_from(&input.data_tables)?;
            plugin_requests
                .entry(cmd.plugin.clone())
                .or_default()
                .insert(dt_id.clone(), df_ids.clone());
        }
        info!(
            "recieved requests for {} plugins: {:?}",
            plugin_requests.len(),
            plugin_requests.keys()
        );

        let requests = plugin_requests
            .iter()
            .map(|(plugin, query)| {
                Ok((
                    plugin,
                    plugins
                        .get(plugin)
                        .ok_or(Error::MissingPlugin(plugin.clone()))?
                        .run_queries(input, query),
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        let requests = stream::iter(requests)
            .map(|(plugin, fut)| async move { (plugin, fut.await) });

        let data = assert_send(requests)
            .buffer_unordered(plugin_requests.len())
            .collect::<Vec<_>>()
            .await;
        info!("API requests done");

        let results =
            data.into_iter()
                .fold(HashMap::new(), |mut accum, (plugin, dm)| {
                    match dm {
                        Ok(dm) => accum.extend(dm),
                        Err(e) => accum.extend(fail_all(
                            plugin_requests
                                .get(plugin)
                                .unwrap()
                                .keys()
                                .collect(),
                            e,
                        )),
                    }
                    accum
                });

        Ok(results)
    }
}

fn fail_all(dts: HashSet<&ProtoDataTableId>, err: Error) -> DataMap {
    warn!("plugi error occured for the following tables: {:?}", &dts);
    dts.into_iter()
        .map(|dt| (dt.clone(), Err(DTError::Plugin(err.to_string()))))
        .collect()
}

// https://github.com/rust-lang/rust/issues/102211
fn assert_send<R>(
    fut: impl futures::Stream<Item = R>,
) -> impl futures::Stream<Item = R> {
    fut
}
