/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use agent_utils::TryAppend;
use async_trait::async_trait;
use etc_base::{ProtoDataFieldId, ProtoDataTableId, ProtoQueryMap};
use protocol::{
    ConfigRef, DataFieldSpec, DataTableSpec, InputRef, LocalPlugin,
    ProtoJsonDataMap, ProtocolProto, ProtocolService,
};
use rpc::{GenericValue, SessionHandler};
use serde_json::value::RawValue;

use super::error::Error;

pub struct ProtocolDaemon<T: LocalPlugin> {
    plugin: T,
}

pub struct Session<T: LocalPlugin> {
    inputs: RwLock<HashMap<InputRef, Arc<T::Input>>>,
    configs: RwLock<HashMap<ConfigRef, Arc<T::Config>>>,
}

impl<T: LocalPlugin> ProtocolDaemon<T> {
    pub fn new(plugin: T) -> Self {
        Self { plugin }
    }
}

#[async_trait]
impl<V, S, T> SessionHandler<ProtocolProto, V, S> for ProtocolDaemon<T>
where
    V: GenericValue,
    S: Send + Sync,
    T: LocalPlugin + 'static,
{
    type Session = Session<T>;
    type Error = Error<T::Error, T::TypeError>;
    async fn session(&self, _info: &S) -> Result<Self::Session, Self::Error> {
        Ok(Session {
            inputs: RwLock::new(HashMap::new()),
            configs: RwLock::new(HashMap::new()),
        })
    }
}

#[async_trait]
impl<T: LocalPlugin + 'static> ProtocolService for ProtocolDaemon<T> {
    type Session = Session<T>;
    type Error = Error<T::Error, T::TypeError>;

    async fn protocol(
        &self,
        _session: &Self::Session,
    ) -> Result<String, Self::Error> {
        Ok(String::from(T::PROTOCOL))
    }

    async fn version(
        &self,
        _session: &Self::Session,
    ) -> Result<String, Self::Error> {
        Ok(String::from(T::VERSION))
    }

    async fn load_inputs(
        &self,
        session: &Self::Session,
        inputs: Vec<Box<RawValue>>,
    ) -> Result<InputRef, Self::Error> {
        let mut input = T::Input::default();
        for val in inputs.into_iter() {
            input
                .try_append(
                    serde_json::from_str(val.get())
                        .map_err(Error::DecodeInput)?,
                )
                .map_err(Error::AppendInput)?
        }

        let id = InputRef::new();
        session.inputs.write().unwrap().insert(id, Arc::new(input));
        Ok(id)
    }

    async fn unload_inputs(
        &self,
        session: &Self::Session,
        input: InputRef,
    ) -> Result<(), Self::Error> {
        session.inputs.write().unwrap().remove(&input);
        Ok(())
    }

    async fn load_config(
        &self,
        session: &Self::Session,
        config: Box<RawValue>,
    ) -> Result<ConfigRef, Self::Error> {
        let id = ConfigRef::new();
        session.configs.write().unwrap().insert(
            id,
            Arc::new(
                serde_json::from_str(config.get())
                    .map_err(Error::DecodeConfig)?,
            ),
        );
        Ok(id)
    }

    async fn unload_config(
        &self,
        session: &Self::Session,
        config: ConfigRef,
    ) -> Result<(), Self::Error> {
        session.configs.write().unwrap().remove(&config);
        Ok(())
    }

    async fn show_queries(
        &self,
        session: &Self::Session,
        query: ProtoQueryMap,
        input: InputRef,
        _config: ConfigRef,
    ) -> Result<String, Self::Error> {
        self.plugin
            .show_queries(
                &session
                    .inputs
                    .read()
                    .unwrap()
                    .get(&input)
                    .ok_or(Error::MissingInput)?
                    .clone(),
                &query,
            )
            .map_err(Error::Plugin)
    }

    async fn run_queries(
        &self,
        session: &Self::Session,
        query: ProtoQueryMap,
        input: InputRef,
        config: ConfigRef,
    ) -> Result<ProtoJsonDataMap, Self::Error> {
        let input = session
            .inputs
            .read()
            .unwrap()
            .get(&input)
            .ok_or(Error::MissingInput)?
            .clone();
        let config = session
            .configs
            .read()
            .unwrap()
            .get(&config)
            .ok_or(Error::MissingConfig)?
            .clone();

        Ok(self
            .plugin
            .run_queries(&input, &config, &query)
            .await
            .map_err(Error::Plugin)?
            .into_iter()
            .map(|(table_id, res)| {
                (
                    table_id,
                    res.map_err(|e| e.to_string()).map(|res| {
                        res.map_warning(|w| w.to_string()).map(|rows| {
                            rows.into_iter()
                                .map(|row| {
                                    row.into_iter()
                                        .map(|(field_id, field_res)| {
                                            (
                                                field_id,
                                                field_res
                                                    .map_err(|e| e.to_string())
                                                    .and_then(|val| {
                                                        val.to_json_value_res()
                                                    }),
                                            )
                                        })
                                        .collect()
                                })
                                .collect()
                        })
                    }),
                )
            })
            .collect())

        // .into_iter()
        // .map(|(table_id, res)| {
        //     (
        //         DataTableId(proto.clone(), table_id.clone()),
        //         res.map(|res| {
        //             res.map(|rows| {
        //                 rows.into_iter()
        //                     .map(|row| {
        //                         row.into_iter()
        //                             .map(|(field_id, field)| {
        //                                 (
        //                                     DataFieldId(
        //                                         proto.clone(),
        //                                         field_id,
        //                                     ),
        //                                     field,
        //                                 )
        //                             })
        //                             .collect()
        //                     })
        //                     .collect()
        //             })
        //             .map_warning(|w| {
        //                 Arc::new(DataTableError {
        //                     origin: ErrorOrigin::DataTable(DataTableId(
        //                         proto.clone(),
        //                         table_id.clone(),
        //                     )),
        //                     error: Box::new(w),
        //                 })
        //             })
        //         })
        //         .map_err(|e| {
        //             Arc::new(DataTableError {
        //                 origin: ErrorOrigin::DataTable(DataTableId(
        //                     proto.clone(),
        //                     table_id.clone(),
        //                 )),
        //                 error: Box::new(e),
        //             })
        //         }),
        //     )
        // })
        // .collect())
    }

    async fn get_tables(
        &self,
        session: &Self::Session,
        input: InputRef,
    ) -> Result<HashMap<ProtoDataTableId, DataTableSpec>, Self::Error> {
        self.plugin
            .get_tables(
                &session
                    .inputs
                    .read()
                    .unwrap()
                    .get(&input)
                    .ok_or(Error::MissingInput)?
                    .clone(),
            )
            .map_err(Error::PluginType)
    }

    async fn get_fields(
        &self,
        session: &Self::Session,
        input: InputRef,
    ) -> Result<HashMap<ProtoDataFieldId, DataFieldSpec>, Self::Error> {
        self.plugin
            .get_fields(
                &session
                    .inputs
                    .read()
                    .unwrap()
                    .get(&input)
                    .ok_or(Error::MissingInput)?
                    .clone(),
            )
            .map_err(Error::PluginType)
    }
}
