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
        create_bool_data, create_data_with_counter_db, create_int_data,
    },
    input::FieldSpec,
};

use super::generic::{CreateTabledata, ValueSoap};

#[derive(Debug, Deserialize)]
pub struct BodyGlobal {
    #[serde(rename = "GetGlobalResponse")]
    pub response: GlobalResponse,
}

#[derive(Debug, Deserialize)]
pub struct GlobalResponse {
    #[serde(rename = "GetGlobalResult")]
    pub result: GlobalResult,
}

#[derive(Debug, Deserialize)]
pub struct GlobalResult {
    #[serde(rename = "RefLocal")]
    pub ref_local: Option<ValueSoap<u64>>,
    #[serde(rename = "RefUpdateLocal")]
    pub ref_update_local: Option<ValueSoap<u64>>,
    #[serde(rename = "RefPrivate")]
    pub ref_private: Option<ValueSoap<u64>>,
    #[serde(rename = "RefUpdatePrivate")]
    pub ref_update_private: Option<ValueSoap<u64>>,
    #[serde(rename = "RefRemote")]
    pub ref_remote: Option<ValueSoap<u64>>,
    #[serde(rename = "RefUpdateRemote")]
    pub ref_update_remote: Option<ValueSoap<u64>>,
    #[serde(rename = "LogicalBlocks")]
    pub logical_blocks: Option<ValueSoap<u64>>,
    #[serde(rename = "PhysBlockReads")]
    pub phys_block_reads: Option<ValueSoap<u64>>,
    #[serde(rename = "PhysBlockWrites")]
    pub phys_block_writes: Option<ValueSoap<u64>>,
    #[serde(rename = "WIJWrites")]
    pub wij_writes: Option<ValueSoap<u64>>,
    #[serde(rename = "ThrottleCnt")]
    pub throttle_count: Option<ValueSoap<u64>>,
    #[serde(rename = "ThrottleCur")]
    pub throttle_current: Option<ValueSoap<i64>>,
    #[serde(rename = "ThrottleMax")]
    pub throttle_max: Option<ValueSoap<i64>>,
    #[serde(rename = "UpdateCnt")]
    pub update_count: Option<ValueSoap<i64>>,
    #[serde(rename = "UpdateLock")]
    pub update_lock: Option<ValueSoap<bool>>,
    #[serde(rename = "JrnEntries")]
    pub journal_entries: Option<ValueSoap<u64>>,
    #[serde(rename = "JrnBlocks")]
    pub journal_blocks: Option<ValueSoap<u64>>,
    #[serde(rename = "WDWake")]
    pub wd_wake: Option<ValueSoap<i64>>,
    #[serde(rename = "WDQueueSize")]
    pub wd_queuesize: Option<ValueSoap<i64>>,
}

impl CreateTabledata for BodyGlobal {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let item = &self.response.result;
        let row: HashMap<ProtoDataFieldId, Data> = fields
            .into_iter()
            .map(|(id, field)| match field.parameter_name.as_str() {
                "RefLocal" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.ref_local,
                    &counterdb,
                    &field,
                ),
                "RefUpdateLocal" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.ref_update_local,
                    &counterdb,
                    &field,
                ),
                "RefPrivate" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.ref_private,
                    &counterdb,
                    &field,
                ),
                "RefUpdatePrivate" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.ref_update_private,
                    &counterdb,
                    &field,
                ),
                "RefRemote" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.ref_remote,
                    &counterdb,
                    &field,
                ),
                "RefUpdateRemote" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.ref_update_remote,
                    &counterdb,
                    &field,
                ),
                "LogicalBlocks" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.logical_blocks,
                    &counterdb,
                    &field,
                ),
                "PhysBlockReads" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.phys_block_reads,
                    &counterdb,
                    &field,
                ),
                "PhysBlockWrites" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.phys_block_writes,
                    &counterdb,
                    &field,
                ),
                "WIJWrites" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.wij_writes,
                    &counterdb,
                    &field,
                ),
                "ThrottleCnt" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.throttle_count,
                    &counterdb,
                    &field,
                ),
                "ThrottleCur" => create_int_data(&id, &item.throttle_current),
                "ThrottleMax" => create_int_data(&id, &item.throttle_max),
                "UpdateCnt" => create_int_data(&id, &item.update_count),
                "UpdateLock" => create_bool_data(&id, &item.update_lock),
                "JrnEntries" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.journal_entries,
                    &counterdb,
                    &field,
                ),
                "JrnBlocks" => create_data_with_counter_db(
                    &id,
                    field.parameter_name.clone(),
                    &item.journal_blocks,
                    &counterdb,
                    &field,
                ),
                "WDWake" => create_int_data(&id, &item.wd_wake),
                "WDQueueSize" => create_int_data(&id, &item.wd_queuesize),
                _ => (id.clone(), Err(value::DataError::Missing)),
            })
            .collect();
        vec![row]
    }
}
