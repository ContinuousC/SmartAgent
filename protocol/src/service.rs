/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use rpc::rpc;

use etc_base::{
    AnnotatedResult, ProtoDataFieldId, ProtoDataTableId, ProtoJsonRow,
    ProtoQueryMap,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use uuid::Uuid;

use crate::{DataFieldSpec, DataTableSpec};

#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct InputRef(Uuid);
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct ConfigRef(Uuid);

pub type ProtoJsonDataMap = HashMap<
    ProtoDataTableId,
    AnnotatedResult<Vec<ProtoJsonRow>, String, String>,
>;

#[rpc(service(session, python, javascript), stub)]
pub trait ProtocolService {
    async fn protocol(&self) -> String;
    async fn version(&self) -> String;

    async fn load_inputs(&self, input: Vec<Box<RawValue>>) -> InputRef;
    async fn unload_inputs(&self, input: InputRef);

    async fn load_config(&self, config: Box<RawValue>) -> ConfigRef;
    async fn unload_config(&self, config: ConfigRef);

    async fn show_queries(
        &self,
        query: ProtoQueryMap,
        input: InputRef,
        config: ConfigRef,
    ) -> String;

    async fn run_queries(
        &self,
        query: ProtoQueryMap,
        input: InputRef,
        config: ConfigRef,
    ) -> ProtoJsonDataMap;

    async fn get_tables(
        &self,
        input: InputRef,
    ) -> HashMap<ProtoDataTableId, DataTableSpec>;
    async fn get_fields(
        &self,
        input: InputRef,
    ) -> HashMap<ProtoDataFieldId, DataFieldSpec>;
}

impl InputRef {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl ConfigRef {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}
