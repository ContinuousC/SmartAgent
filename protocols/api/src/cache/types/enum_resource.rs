/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, sync::Arc};

use etc_base::ProtoDataFieldId;
use protocol::CounterDb;
use serde::Deserialize;
use std::sync::Mutex;
use value::{Data, Value};

use crate::input::FieldSpec;

use super::generic::{create_data_with_counter_db, CreateTabledata, ValueSoap};

#[derive(Debug, Deserialize)]
pub struct BodyEnumResource {
    #[serde(rename = "EnumResourceResponse")]
    pub response: EnumResourceResponse,
}

#[derive(Debug, Deserialize)]
pub struct EnumResourceResponse {
    #[serde(rename = "EnumResourceResult")]
    pub result: EnumResourceResult,
}

#[derive(Debug, Deserialize)]
pub struct EnumResourceResult {
    #[serde(rename = "diffgram")]
    pub diffgr_diffgram: EnumResourcediffgram,
}

#[derive(Debug, Deserialize)]
pub struct EnumResourcediffgram {
    #[serde(rename = "DefaultDataSet")]
    pub data_set: EnumResourceDataSet,
}

#[derive(Debug, Deserialize)]
pub struct EnumResourceDataSet {
    #[serde(rename = "Sample")]
    pub samples: Vec<EnumResourceSample>,
}

#[derive(Debug, Deserialize)]
pub struct EnumResourceSample {
    #[serde(rename = "Name")]
    pub name: ValueSoap<String>,
    #[serde(rename = "Seize")]
    pub seize: Option<ValueSoap<u64>>,
    #[serde(rename = "Nseize")]
    pub nseize: Option<ValueSoap<u64>>,
    #[serde(rename = "Aseize")]
    pub aseize: Option<ValueSoap<u64>>,
    #[serde(rename = "Bseize")]
    pub bseize: Option<ValueSoap<u64>>,
    #[serde(rename = "BusySet")]
    pub busy_set: Option<ValueSoap<u64>>,
}

impl CreateTabledata for BodyEnumResource {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let mut samples_vec: Vec<HashMap<ProtoDataFieldId, Data>> =
            Default::default();
        let items = &self.response.result.diffgr_diffgram.data_set.samples;

        for item in items {
            let row: HashMap<ProtoDataFieldId, Data> = fields
                .iter()
                .map(|(id, field)| match field.parameter_name.as_str() {
                    "Name" => (
                        id.clone(),
                        Ok(Value::UnicodeString(item.name.value.clone())),
                    ),

                    "Seize" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            item.name.value,
                            field.parameter_name.clone()
                        ),
                        &item.seize,
                        &counterdb,
                        field,
                    ),
                    "Nseize" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            item.name.value,
                            field.parameter_name.clone()
                        ),
                        &item.nseize,
                        &counterdb,
                        field,
                    ),
                    "Aseize" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            item.name.value,
                            field.parameter_name.clone()
                        ),
                        &item.aseize,
                        &counterdb,
                        field,
                    ),
                    "Bseize" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            item.name.value,
                            field.parameter_name.clone()
                        ),
                        &item.bseize,
                        &counterdb,
                        field,
                    ),
                    "BusySet" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            item.name.value,
                            field.parameter_name.clone()
                        ),
                        &item.busy_set,
                        &counterdb,
                        field,
                    ),
                    _ => (id.clone(), Err(value::DataError::Missing)),
                })
                .collect();
            samples_vec.push(row);
        }
        samples_vec
    }
}
