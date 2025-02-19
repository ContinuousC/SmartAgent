/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, sync::Arc};

use etc_base::ProtoDataFieldId;
use protocol::CounterDb;
use serde::Deserialize;
use std::sync::Mutex;
use value::Data;

use super::generic::{
    create_bool_data, create_int_data, create_string_data, CreateTabledata,
    ValueSoap,
};
use crate::input::FieldSpec;

#[derive(Debug, Deserialize)]
pub struct BodyEnumDatabase {
    #[serde(rename = "EnumDatabaseResponse")]
    pub response: EnumDatabaseResponse,
}
#[derive(Debug, Deserialize)]
pub struct EnumDatabaseResponse {
    #[serde(rename = "EnumDatabaseResult")]
    pub result: EnumDatabaseResult,
}

#[derive(Debug, Deserialize)]
pub struct EnumDatabaseResult {
    #[serde(rename = "diffgram")]
    pub diffgr_diffgram: EnumDatabasediffgram,
}

#[derive(Debug, Deserialize)]
pub struct EnumDatabasediffgram {
    #[serde(rename = "DefaultDataSet")]
    pub data_set: EnumDatabaseDataSet,
}

#[derive(Debug, Deserialize)]
pub struct EnumDatabaseDataSet {
    #[serde(rename = "List")]
    pub list: Vec<EnumDatabaseList>,
}

#[derive(Debug, Deserialize)]
pub struct EnumDatabaseList {
    #[serde(rename = "Name")]
    pub name: Option<ValueSoap<String>>,
    #[serde(rename = "Directory")]
    pub directory: Option<ValueSoap<String>>,
    #[serde(rename = "SizeAllocated")]
    pub size_allocated: Option<ValueSoap<i64>>,
    #[serde(rename = "Mounted")]
    pub mounted: Option<ValueSoap<bool>>,
    #[serde(rename = "ReadOnly")]
    pub read_only: Option<ValueSoap<bool>>,
    #[serde(rename = "Cluster")]
    pub cluster: Option<ValueSoap<bool>>,
    #[serde(rename = "FreeSpace")]
    pub free_space: Option<ValueSoap<i64>>,
}

impl CreateTabledata for BodyEnumDatabase {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let mut samples_vec: Vec<HashMap<ProtoDataFieldId, Data>> =
            Default::default();
        let list = self.response.result.diffgr_diffgram.data_set.list;

        for item in &list {
            let row: HashMap<ProtoDataFieldId, Data> = fields
                .clone()
                .iter()
                .map(|(id, field)| match field.parameter_name.as_str() {
                    "Name" => create_string_data(id, &item.name),
                    "Directory" => create_string_data(id, &item.directory),
                    "SizeAllocated" => {
                        create_int_data(id, &item.size_allocated)
                    }
                    "Mounted" => create_bool_data(id, &item.mounted),
                    "ReadOnly" => create_bool_data(id, &item.read_only),
                    "Cluster" => create_bool_data(id, &item.cluster),
                    "FreeSpace" => create_int_data(id, &item.free_space),
                    _ => (id.clone(), Err(value::DataError::Missing)),
                })
                .collect();
            samples_vec.push(row);
        }
        samples_vec
    }
}
