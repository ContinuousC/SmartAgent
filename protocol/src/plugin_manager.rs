/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

// use log::debug;
use serde_json::value::RawValue;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use etc_base::{DataFieldId, DataTableId, Protocol, QueryMap};

use super::error::{DataTableError, Error, ErrorOrigin, Result};
use super::generic_plugin::{DataMap, GenericPlugin};
use super::input::Input;
use super::local_plugin::LocalPlugin;

pub struct PluginManager {
    plugins: HashMap<Protocol, Box<dyn GenericPlugin + Send + Sync>>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn add_plugin<T: GenericPlugin + Send + Sync + 'static>(
        &mut self,
        plugin: T,
    ) {
        self.plugins.insert(plugin.protocol(), Box::new(plugin));
    }

    pub fn remove_plugin(&mut self, proto: &Protocol) {
        self.plugins.remove(proto);
    }

    pub fn get_protocols(&self) -> HashSet<Protocol> {
        self.plugins.keys().cloned().collect()
    }

    pub fn get_local_plugin<T: LocalPlugin + 'static>(&self) -> Result<&T> {
        let proto = Protocol(String::from(T::PROTOCOL));
        self.plugins
            .get(&proto)
            .ok_or_else(|| Error::MissingPlugin(proto.clone()))?
            .as_any()
            .downcast_ref()
            .ok_or_else(|| Error::MissingPlugin(proto.clone()))
    }

    pub async fn load_inputs(
        &self,
        mut inputs: Vec<HashMap<Protocol, Box<RawValue>>>,
    ) -> Result<HashMap<Protocol, Input>> {
        let mut input_map = HashMap::new();
        let protos: HashSet<Protocol> =
            inputs.iter().flat_map(|i| i.keys().cloned()).collect();

        for proto in &protos {
            // debug!("loading input for {:?}", &self.plugins.keys());
            if let Some(plugin) = self.plugins.get(proto) {
                // debug!("loading input for {}", &proto);
                let proto_inputs =
                    inputs.iter_mut().filter_map(|i| i.remove(proto)).collect();
                input_map.insert(
                    proto.clone(),
                    plugin.load_inputs(proto_inputs).await?,
                );
            }
        }

        Ok(input_map)
    }

    pub fn show_queries(
        &self,
        input: &HashMap<Protocol, Input>,
        prot_queries: &QueryMap,
    ) {
        for (proto, proto_query) in prot_queries {
            match self.plugins.get(proto) {
                None => println!("Plugin for protocol {} not found", proto),
                Some(plugin) => match input.get(proto) {
                    None => println!("Input for protocol {} not found", proto),
                    Some(proto_input) => match plugin
                        .show_queries(proto_input.handle.as_ref(), proto_query)
                    {
                        Ok(queries) => {
                            println!("Queries for {}:\n{}", &proto, queries)
                        }
                        Err(e) => println!(
                            "Error showing queries for {}: {}",
                            &proto, e
                        ),
                    },
                },
            }
        }
    }

    pub async fn run_queries(
        &self,
        input: &HashMap<Protocol, Input>,
        mut config: HashMap<Protocol, Box<RawValue>>,
        query: &QueryMap,
    ) -> Result<DataMap> {
        let mut data_map = HashMap::new();

        for (proto, proto_query) in query {
            let plugin = self
                .plugins
                .get(proto)
                .ok_or_else(|| Error::MissingPlugin(proto.clone()))?;
            let proto_input = input
                .get(proto)
                .ok_or_else(|| Error::MissingInput(proto.clone()))?
                .handle
                .as_ref();
            let proto_config = config
                .remove(proto)
                .ok_or_else(|| Error::MissingConfig(proto.clone()))?;

            let proto_res = plugin
                .run_queries(proto_input, &proto_config, proto_query)
                .await;

            match proto_res {
                Ok(mut data) => {
                    for data_table_id in proto_query.keys() {
                        data_map.insert(
                            DataTableId(proto.clone(), data_table_id.clone()),
                            data.remove(data_table_id)
                                .unwrap_or_else(|| {
                                    Err(Arc::new(DataTableError {
                                        origin: ErrorOrigin::Protocol(
                                            proto.clone(),
                                        ),
                                        error: Box::new(
                                            MgrError::MissingDataTable,
                                        ),
                                    }))
                                })
                                .map(|table_res| {
                                    table_res.map(|rows| {
                                        rows.into_iter()
                                            .map(|row| {
                                                row
															 .into_iter()
															 .map(|(field_id,field_res)|{
																 (DataFieldId(proto.clone(),field_id),field_res)
															 })
															 .collect()
                                            })
                                            .collect()
                                    })
                                }),
                        );
                    }
                }
                Err(err) => {
                    let error = Arc::new(DataTableError {
                        origin: ErrorOrigin::Protocol(proto.clone()),
                        error: Box::new(MgrError::PluginFailed(Box::new(err))),
                    });
                    for data_table_id in proto_query.keys() {
                        data_map.insert(
                            DataTableId(proto.clone(), data_table_id.clone()),
                            Err(error.clone()),
                        );
                    }
                }
            }
        }

        Ok(data_map)
    }
}

#[derive(thiserror::Error, Debug)]
enum MgrError {
    #[error("Data table missing in plugin output")]
    MissingDataTable,
    #[error("Protocol plugin failure: {0}")]
    PluginFailed(Box<dyn std::error::Error + Send + Sync + 'static>),
}
