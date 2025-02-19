/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use agent_utils::TryAppend;
use async_trait::async_trait;
use etc_base::{
    AnnotatedResult, DataTableId, ProtoDataTableId, ProtoQueryMap, ProtoRow,
    Protocol, Row,
};
use serde_json::value::RawValue;

use super::error::{DataTableError, Error, ErrorOrigin, Result};
use super::input::Input;
use super::local_plugin::LocalPlugin;

pub type DataMap = HashMap<
    DataTableId,
    AnnotatedResult<Vec<Row>, Arc<DataTableError>, Arc<DataTableError>>,
>;

pub type ProtoDataMap = HashMap<
    ProtoDataTableId,
    AnnotatedResult<Vec<ProtoRow>, Arc<DataTableError>, Arc<DataTableError>>,
>;

/// Type-erased protocol plugin.
#[async_trait]
pub trait GenericPlugin {
    fn as_any(&self) -> &dyn Any;

    fn protocol(&self) -> Protocol;
    async fn version(&self) -> String;

    async fn load_inputs(&self, input: Vec<Box<RawValue>>) -> Result<Input>;

    fn show_queries(
        &self,
        input: &(dyn Any + Send + Sync),
        query: &ProtoQueryMap,
    ) -> Result<String>;

    async fn run_queries(
        &self,
        input: &(dyn Any + Send + Sync),
        config: &RawValue,
        query: &ProtoQueryMap,
    ) -> Result<ProtoDataMap>;
}

/// Implementation for compiled-in plugins.
#[async_trait]
impl<T: LocalPlugin + 'static> GenericPlugin for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn protocol(&self) -> Protocol {
        Protocol(String::from(T::PROTOCOL))
    }

    async fn version(&self) -> String {
        String::from(T::VERSION)
    }

    async fn load_inputs(&self, inputs: Vec<Box<RawValue>>) -> Result<Input> {
        let mut input = T::Input::default();
        for val in inputs.into_iter() {
            input
                .try_append(
                    serde_path_to_error::deserialize(
                        &mut serde_json::Deserializer::from_str(val.get()),
                    )
                    .map_err(|e| Error::InputFormat(self.protocol(), e))?,
                )
                .map_err(|e| Error::Plugin(self.protocol(), Box::new(e)))?;
        }
        Ok(Input {
            data_tables: self
                .get_tables(&input)
                .map_err(|e| Error::Plugin(self.protocol(), Box::new(e)))?,
            data_fields: self
                .get_fields(&input)
                .map_err(|e| Error::Plugin(self.protocol(), Box::new(e)))?,
            handle: Box::new(input),
        })
    }

    fn show_queries(
        &self,
        input: &(dyn Any + Send + Sync),
        prot_queries: &ProtoQueryMap,
    ) -> Result<String> {
        let input = input
            .downcast_ref()
            .ok_or_else(|| Error::WrongInput(self.protocol()))?;
        self.show_queries(input, prot_queries)
            .map_err(|e| Error::Plugin(self.protocol(), Box::new(e)))
    }

    async fn run_queries(
        &self,
        input: &(dyn Any + Send + Sync),
        config: &RawValue,
        query: &ProtoQueryMap,
    ) -> Result<ProtoDataMap> {
        let input = input
            .downcast_ref()
            .ok_or_else(|| Error::WrongInput(self.protocol()))?;
        let config = serde_json::from_str(config.get())
            .map_err(|e| Error::ConfigFormat(self.protocol(), e))?;

        let result = self
            .run_queries(input, &config, query)
            .await
            .map_err(|e| Error::Plugin(self.protocol(), Box::new(e)))?;
        Ok(make_data_map::<T>(result))
    }
}

fn make_data_map<T: LocalPlugin>(
    data: HashMap<
        ProtoDataTableId,
        AnnotatedResult<Vec<ProtoRow>, T::DTWarning, T::DTError>,
    >,
) -> ProtoDataMap {
    let proto = Protocol(String::from(T::PROTOCOL));
    data.into_iter()
        .map(|(table_id, res)| {
            (
                table_id.clone(),
                res.map(|res| {
                    res.map_warning(|w| {
                        Arc::new(DataTableError {
                            origin: ErrorOrigin::DataTable(DataTableId(
                                proto.clone(),
                                table_id.clone(),
                            )),
                            error: Box::new(w),
                        })
                    })
                })
                .map_err(|e| {
                    Arc::new(DataTableError {
                        origin: ErrorOrigin::DataTable(DataTableId(
                            proto.clone(),
                            table_id.clone(),
                        )),
                        error: Box::new(e),
                    })
                }),
            )
        })
        .collect()
}
