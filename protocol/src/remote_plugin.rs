/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serde_json::value::RawValue;
use thiserror::Error;

use etc_base::{DataTableId, ProtoDataFieldId, ProtoQueryMap, Protocol};
use value::{DataError, Type};

use crate::{
    error::{Error, Result},
    service::{ProtocolProto, ProtocolServiceStub},
    DataTableError, ErrorOrigin, GenericPlugin, InputRef, ProtoDataMap,
};

pub struct RemotePlugin<T> {
    plugin: T,
    protocol: Protocol,
    version: String,
}

struct Input {
    remote: InputRef,
    types: HashMap<ProtoDataFieldId, Type>,
}

impl<T, V> RemotePlugin<ProtocolServiceStub<T, V>>
where
    T: rpc::RequestHandler<ProtocolProto, V, ExtraArgs = ()>,
    V: rpc::GenericValue + Send,
    V::Error: Send + Sync,
    T::Error: std::fmt::Display + Send + Sync,
{
    pub async fn new(plugin: ProtocolServiceStub<T, V>) -> Result<Self> {
        let protocol =
            Protocol(plugin.protocol().await.map_err(Error::RemotePluginInit)?);
        let version = plugin
            .version()
            .await
            .map_err(|e| Error::RemotePlugin(protocol.clone(), e))?;

        Ok(Self {
            plugin,
            protocol,
            version,
        })
    }
}

#[async_trait]
impl<T, V> GenericPlugin for RemotePlugin<ProtocolServiceStub<T, V>>
where
    T: rpc::RequestHandler<ProtocolProto, V, ExtraArgs = ()> + Send + Sync,
    V: rpc::GenericValue + Send + Sync,
    V::Error: Send + Sync,
    T::Error: std::fmt::Display + Send + Sync,
{
    fn as_any(&self) -> &dyn std::any::Any {
        todo!()
    }

    fn protocol(&self) -> Protocol {
        self.protocol.clone()
    }
    async fn version(&self) -> String {
        self.version.clone()
    }

    async fn load_inputs(
        &self,
        input: Vec<Box<RawValue>>,
    ) -> Result<crate::Input> {
        let input = self
            .plugin
            .load_inputs(input)
            .await
            .map_err(|e| Error::RemotePlugin(self.protocol.clone(), e))?;
        let data_tables = self
            .plugin
            .get_tables(input)
            .await
            .map_err(|e| Error::RemotePlugin(self.protocol.clone(), e))?;
        let data_fields = self
            .plugin
            .get_fields(input)
            .await
            .map_err(|e| Error::RemotePlugin(self.protocol.clone(), e))?;
        Ok(crate::Input {
            handle: Box::new(Input {
                remote: input,
                types: data_fields
                    .iter()
                    .map(|(field_id, field_spec)| {
                        (field_id.clone(), field_spec.input_type.clone())
                    })
                    .collect(),
            }),
            data_tables,
            data_fields,
        })
    }

    fn show_queries(
        &self,
        _input: &(dyn std::any::Any + Send + Sync),
        _query: &ProtoQueryMap,
    ) -> crate::error::Result<String> {
        todo!()
    }

    async fn run_queries(
        &self,
        input: &(dyn std::any::Any + Send + Sync),
        config: &RawValue,
        query: &ProtoQueryMap,
    ) -> crate::error::Result<ProtoDataMap> {
        let input: &Input = input
            .downcast_ref()
            .ok_or_else(|| crate::Error::WrongInput(self.protocol.clone()))?;
        let config = self
            .plugin
            .load_config(config.to_owned())
            .await
            .map_err(|e| Error::RemotePlugin(self.protocol.clone(), e))?;
        let res = self
            .plugin
            .run_queries(query.clone(), input.remote, config)
            .await
            .map_err(|e| Error::RemotePlugin(self.protocol.clone(), e));
        self.plugin
            .unload_config(config)
            .await
            .map_err(|e| Error::RemotePlugin(self.protocol.clone(), e))?;
        res.map(|r| {
            r.into_iter()
                .map(|(table_id, table_res)| {
                    (
                        table_id.clone(),
                        table_res
                            .map_err(|e| Arc::new(DataTableError {
                                origin: ErrorOrigin::DataTable(DataTableId(
                                    self.protocol().clone(),
                                    table_id.clone(),
                                )),
                                error: Box::new(RemoteError(e)),
                            }))
                            .map(|r| {
                                r
                                    .map_warning(|w| {
										Arc::new( DataTableError {
											origin: ErrorOrigin::DataTable(DataTableId(
												self.protocol().clone(),
												table_id.clone(),
											)),
											error: Box::new(RemoteError(w)),
										})
									})
									.map(|rows| {
                                    rows
										.into_iter()
										.map(|row| {
											row
												.into_iter()
												.map(|(field_id, field_res)| {
														(
															field_id.clone(),
															field_res
																.map_err(DataError::External)
																.and_then(
																|val| -> std::result::Result<value::Value,DataError> {
																	input.types
																		.get(&field_id)
																		.ok_or_else(||DataError::TypeError("field not found in input".to_string()))?.value_from_json(val)
																}
															),
														)
												})
												.collect()
										})
										.collect()
                                })
                            }),
                    )
                })
                .collect()
        })
    }
}

#[derive(Error, Debug)]
#[error("{0}")]
struct RemoteError(String);
