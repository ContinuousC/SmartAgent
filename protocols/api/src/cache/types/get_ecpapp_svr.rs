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
    cache::types::generic::{
        create_data_with_counter_db, create_float_data, create_int_data,
    },
    input::FieldSpec,
};

use super::generic::{CreateTabledata, ValueSoap};

#[derive(Debug, Deserialize)]
pub struct BodyECPAppSvr {
    #[serde(rename = "GetECPAppSvrResponse")]
    pub response: ECPAppSvrResponse,
}

#[derive(Debug, Deserialize)]
pub struct ECPAppSvrResponse {
    #[serde(rename = "GetECPAppSvrResult")]
    pub result: ECPAppSvrResult,
}

#[derive(Debug, Deserialize)]
pub struct ECPAppSvrResult {
    #[serde(rename = "MaxConn")]
    pub max_conn: Option<ValueSoap<i64>>,
    #[serde(rename = "ActConn")]
    pub act_conn: Option<ValueSoap<i64>>,
    #[serde(rename = "GloRef")]
    pub glo_ref: Option<ValueSoap<u64>>,
    #[serde(rename = "ByteSent")]
    pub byte_sent: Option<ValueSoap<u64>>,
    #[serde(rename = "ByteRcvd")]
    pub byte_rcvd: Option<ValueSoap<u64>>,
    #[serde(rename = "BlockAdd")]
    pub block_add: Option<ValueSoap<u64>>,
    #[serde(rename = "BlockBuffPurge")]
    pub block_buff_purge: Option<ValueSoap<u64>>,
    #[serde(rename = "BlockSvrPurge")]
    pub block_svr_purge: Option<ValueSoap<u64>>,
    #[serde(rename = "GloRefLocal")]
    pub glo_ref_local: Option<ValueSoap<u64>>,
    #[serde(rename = "GloRefRemote")]
    pub glo_ref_remote: Option<ValueSoap<u64>>,
    #[serde(rename = "GloUpdateLocal")]
    pub glo_update_local: Option<ValueSoap<u64>>,
    #[serde(rename = "GloUpdateRemote")]
    pub glo_update_remote: Option<ValueSoap<u64>>,
    #[serde(rename = "RoutineCallLocal")]
    pub routine_call_local: Option<ValueSoap<u64>>,
    #[serde(rename = "RoutineCallRemote")]
    pub routine_call_remote: Option<ValueSoap<u64>>,
    #[serde(rename = "RoutineBuffLocal")]
    pub routine_buff_local: Option<ValueSoap<u64>>,
    #[serde(rename = "RoutineBuffRemote")]
    pub routine_buff_remote: Option<ValueSoap<u64>>,
    #[serde(rename = "ResponseTime")]
    pub response_time: Option<ValueSoap<f64>>,
    #[serde(rename = "ResponseConn")]
    pub response_conn: Option<ValueSoap<i64>>,
}

impl CreateTabledata for BodyECPAppSvr {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let item = &self.response.result;

        let row: HashMap<ProtoDataFieldId, Data> = fields
            .into_iter()
            .map(|(id, field)| match field.parameter_name.as_str() {
                "MaxConn" => create_int_data(&id, &item.max_conn),
                "ActConn" => create_int_data(&id, &item.act_conn),
                "GloRef" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.glo_ref,
                    &counterdb,
                    &field,
                ),
                "ByteSent" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.byte_sent,
                    &counterdb,
                    &field,
                ),
                "ByteRcvd" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.byte_rcvd,
                    &counterdb,
                    &field,
                ),
                "BlockAdd" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.block_add,
                    &counterdb,
                    &field,
                ),
                "BlockBuffPurge" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.block_buff_purge,
                    &counterdb,
                    &field,
                ),
                "BlockSvrPurge" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.block_svr_purge,
                    &counterdb,
                    &field,
                ),
                "GloRefLocal" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.glo_ref_local,
                    &counterdb,
                    &field,
                ),
                "GloRefRemote" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.glo_ref_remote,
                    &counterdb,
                    &field,
                ),
                "GloUpdateLocal" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.glo_update_local,
                    &counterdb,
                    &field,
                ),
                "GloUpdateRemote" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.glo_update_remote,
                    &counterdb,
                    &field,
                ),
                "RoutineCallLocal" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_call_local,
                    &counterdb,
                    &field,
                ),
                "RoutineCallRemote" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_call_remote,
                    &counterdb,
                    &field,
                ),
                "RoutineBuffLocal" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_buff_local,
                    &counterdb,
                    &field,
                ),
                "RoutineBuffRemote" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_buff_remote,
                    &counterdb,
                    &field,
                ),
                "ResponseTime" => create_float_data(&id, &item.response_time),
                "ResponseConn" => create_int_data(&id, &item.response_conn),
                _ => (id.clone(), Err(value::DataError::Missing)),
            })
            .collect();
        vec![row]
    }
}
