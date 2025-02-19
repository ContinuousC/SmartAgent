/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, sync::Arc};

use etc_base::ProtoDataFieldId;
use protocol::CounterDb;
use serde::Deserialize;
use std::sync::Mutex;
use value::Data;

use crate::input::FieldSpec;

use super::generic::{
    create_int_data, create_string_data, CreateTabledata, ValueSoap,
};

#[derive(Debug, Deserialize)]
pub struct BodyEnumProcess {
    #[serde(rename = "EnumProcessResponse")]
    pub response: EnumProcessResponse,
}

#[derive(Debug, Deserialize)]
pub struct EnumProcessResponse {
    #[serde(rename = "EnumProcessResult")]
    pub result: EnumProcessResult,
}

#[derive(Debug, Deserialize)]
pub struct EnumProcessResult {
    #[serde(rename = "diffgram")]
    pub diffgr_diffgram: EnumProcessdiffgram,
}

#[derive(Debug, Deserialize)]
pub struct EnumProcessdiffgram {
    #[serde(rename = "DefaultDataSet")]
    pub data_set: EnumProcessDataSet,
}

#[derive(Debug, Deserialize)]
pub struct EnumProcessDataSet {
    #[serde(rename = "List")]
    pub list: Vec<EnumProcessList>,
}

#[derive(Debug, Deserialize)]
pub struct EnumProcessList {
    #[serde(rename = "Process")]
    pub process: Option<ValueSoap<i64>>,
    #[serde(rename = "UserName")]
    pub user_name: Option<ValueSoap<String>>,
    #[serde(rename = "CurrentDevice")]
    pub current_device: Option<ValueSoap<String>>,
    #[serde(rename = "Namespace")]
    pub namespace: Option<ValueSoap<String>>,
    #[serde(rename = "Routine")]
    pub routine: Option<ValueSoap<String>>,
    #[serde(rename = "CommandsExecuted")]
    pub commands_executed: Option<ValueSoap<i64>>,
    #[serde(rename = "GlobalReferences")]
    pub global_references: Option<ValueSoap<i64>>,
    #[serde(rename = "State")]
    pub state: Option<ValueSoap<String>>,
    #[serde(rename = "ClientName")]
    pub client_name: Option<ValueSoap<String>>,
    #[serde(rename = "ClientIPAddress")]
    pub client_ipaddress: Option<ValueSoap<String>>,
}

impl CreateTabledata for BodyEnumProcess {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let mut samples_vec: Vec<HashMap<ProtoDataFieldId, Data>> =
            Default::default();
        let list = &self.response.result.diffgr_diffgram.data_set.list;

        for item in list {
            let row: HashMap<ProtoDataFieldId, Data> = fields
                .iter()
                .map(|(id, field)| match field.parameter_name.as_str() {
                    "Process" => create_int_data(id, &item.process),
                    "UserName" => create_string_data(id, &item.user_name),
                    "CurrentDevice" => {
                        create_string_data(id, &item.current_device)
                    }
                    "Namespace" => create_string_data(id, &item.namespace),
                    "Routine" => create_string_data(id, &item.routine),
                    "CommandsExecuted" => {
                        create_int_data(id, &item.commands_executed)
                    }
                    "GlobalReferences" => {
                        create_int_data(id, &item.global_references)
                    }
                    "State" => create_string_data(id, &item.state),
                    "ClientName" => create_string_data(id, &item.client_name),
                    "clientIPAddress" => {
                        create_string_data(id, &item.client_ipaddress)
                    }
                    _ => (id.clone(), Err(value::DataError::Missing)),
                })
                .collect();
            samples_vec.push(row);
        }
        samples_vec
    }
}
