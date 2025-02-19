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
pub struct BodyECPDataSvr {
    #[serde(rename = "GetECPDataSvrResponse")]
    pub response: ECPDataSvrResponse,
}

#[derive(Debug, Deserialize)]
pub struct ECPDataSvrResponse {
    #[serde(rename = "GetECPDataSvrResult")]
    pub result: ECPDataSvrResult,
}

#[derive(Debug, Deserialize)]
pub struct ECPDataSvrResult {
    #[serde(rename = "MaxConn")]
    pub max_conn: Option<ValueSoap<i64>>,
    #[serde(rename = "ActConn")]
    pub act_conn: Option<ValueSoap<i64>>,
    #[serde(rename = "GloRef")]
    pub glo_ref: Option<ValueSoap<u64>>,
    #[serde(rename = "GloUpdate")]
    pub glo_update: Option<ValueSoap<u64>>,
    #[serde(rename = "ReqRcvd")]
    pub req_rcvd: Option<ValueSoap<u64>>,
    #[serde(rename = "ReqBuff")]
    pub req_buff: Option<ValueSoap<u64>>,
    #[serde(rename = "BlockSent")]
    pub block_sent: Option<ValueSoap<u64>>,
    #[serde(rename = "LockGrant")]
    pub lock_grant: Option<ValueSoap<u64>>,
    #[serde(rename = "LockFail")]
    pub lock_fail: Option<ValueSoap<u64>>,
    #[serde(rename = "LockQue")]
    pub lock_que: Option<ValueSoap<u64>>,
    #[serde(rename = "LockQueGrant")]
    pub lock_que_grant: Option<ValueSoap<u64>>,
    #[serde(rename = "LockQueFail")]
    pub lock_que_fail: Option<ValueSoap<u64>>,
    #[serde(rename = "ByteSent")]
    pub byte_sent: Option<ValueSoap<u64>>,
    #[serde(rename = "ByteRcvd")]
    pub byte_rcvd: Option<ValueSoap<u64>>,
    #[serde(rename = "SvrBlockPurge")]
    pub svr_block_purge: Option<ValueSoap<u64>>,
    #[serde(rename = "RoutinePurge")]
    pub routine_purge: Option<ValueSoap<u64>>,
    #[serde(rename = "BigKill")]
    pub big_kill: Option<ValueSoap<u64>>,
    #[serde(rename = "BigString")]
    pub big_string: Option<ValueSoap<u64>>,
}

impl CreateTabledata for BodyECPDataSvr {
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
                "GloUpdate" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.glo_update,
                    &counterdb,
                    &field,
                ),
                "ReqRcvd" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.req_rcvd,
                    &counterdb,
                    &field,
                ),
                "ReqBuff" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.req_buff,
                    &counterdb,
                    &field,
                ),
                "BlockSent" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.block_sent,
                    &counterdb,
                    &field,
                ),
                "LockGrant" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.lock_grant,
                    &counterdb,
                    &field,
                ),
                "LockFail" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.lock_fail,
                    &counterdb,
                    &field,
                ),
                "LockQue" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.lock_que,
                    &counterdb,
                    &field,
                ),
                "LockQueGrant" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.lock_que_grant,
                    &counterdb,
                    &field,
                ),
                "LockQueFail" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.lock_que_fail,
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
                "SvrBlockPurge" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.svr_block_purge,
                    &counterdb,
                    &field,
                ),
                "RoutinePurge" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.routine_purge,
                    &counterdb,
                    &field,
                ),
                "BigKill" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.big_kill,
                    &counterdb,
                    &field,
                ),
                "BigString" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.big_string,
                    &counterdb,
                    &field,
                ),
                _ => (id.clone(), Err(value::DataError::Missing)),
            })
            .collect();
        vec![row]
    }
}
