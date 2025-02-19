/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, sync::Arc};

use etc_base::ProtoDataFieldId;
use protocol::CounterDb;
use serde::Deserialize;
use std::sync::Mutex;
use value::Data;

use crate::{
    cache::types::generic::{create_data_with_counter_db, create_int_data},
    input::FieldSpec,
};

use super::generic::{CreateTabledata, ValueSoap};

#[derive(Debug, Deserialize)]
pub struct BodyRoutine {
    #[serde(rename = "GetRoutineResponse")]
    pub response: RoutineResponse,
}

#[derive(Debug, Deserialize)]
pub struct RoutineResponse {
    #[serde(rename = "GetRoutineResult")]
    pub result: RoutineResult,
}

#[derive(Debug, Deserialize)]
pub struct RoutineResult {
    #[serde(rename = "RtnLines")]
    pub routine_lines: Option<ValueSoap<i64>>,
    #[serde(rename = "RtnCommands")]
    pub routine_commands: Option<ValueSoap<u64>>,
    #[serde(rename = "RtnCallsLocal")]
    pub routine_calls_local: Option<ValueSoap<u64>>,
    #[serde(rename = "RtnCallsRemote")]
    pub routine_calls_remote: Option<ValueSoap<u64>>,
    #[serde(rename = "RtnFetchLocal")]
    pub routine_fetch_local: Option<ValueSoap<u64>>,
    #[serde(rename = "RtnFetchRemote")]
    pub routine_fetch_remote: Option<ValueSoap<u64>>,
    #[serde(rename = "RtnNotCached")]
    pub routine_not_cached: Option<ValueSoap<u64>>,
}

impl CreateTabledata for BodyRoutine {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let item = &self.response.result;
        let row: HashMap<ProtoDataFieldId, Data> = fields
            .into_iter()
            .map(|(id, field)| match field.parameter_name.as_str() {
                "RtnLines" => create_int_data(&id, &item.routine_lines), // Deprecated
                "RtnCommands" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_commands,
                    &counterdb,
                    &field,
                ),
                "RtnCallsLocal" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_calls_local,
                    &counterdb,
                    &field,
                ),
                "RtnCallsRemote" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_calls_remote,
                    &counterdb,
                    &field,
                ),
                "RtnFetchLocal" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_fetch_local,
                    &counterdb,
                    &field,
                ),
                "RtnFetchRemote" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_fetch_remote,
                    &counterdb,
                    &field,
                ),
                "RtnNotCached" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_not_cached,
                    &counterdb,
                    &field,
                ),
                _ => (id.clone(), Err(value::DataError::Missing)),
            })
            .collect();
        vec![row]
    }
}
