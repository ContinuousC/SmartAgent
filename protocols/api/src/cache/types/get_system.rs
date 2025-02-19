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
    cache::types::generic::{create_int_data, create_string_data},
    input::FieldSpec,
};

use super::generic::{CreateTabledata, ValueSoap};

#[derive(Debug, Deserialize)]
pub struct BodySystem {
    #[serde(rename = "GetSystemResponse")]
    pub response: SystemResponse,
}

#[derive(Debug, Deserialize)]
pub struct SystemResponse {
    #[serde(rename = "GetSystemResult")]
    pub result: SystemResult,
}

#[derive(Debug, Deserialize)]
pub struct SystemResult {
    #[serde(rename = "Name")]
    pub name: Option<ValueSoap<String>>,
    #[serde(rename = "System")]
    pub system: Option<ValueSoap<String>>,
    #[serde(rename = "ConfigFile")]
    pub config_file: Option<ValueSoap<String>>,
    #[serde(rename = "Directory")]
    pub directory: Option<ValueSoap<String>>,
    #[serde(rename = "Version")]
    pub version: Option<ValueSoap<String>>,
    #[serde(rename = "CurrentUsers")]
    pub current_users: Option<ValueSoap<i64>>,
    #[serde(rename = "RoutineCache")]
    pub routine_cache: Option<ValueSoap<i64>>,
    #[serde(rename = "DatabaseCache")]
    pub database_cache: Option<ValueSoap<i64>>,
    #[serde(rename = "LicenseAvailable")]
    pub license_available: Option<ValueSoap<i64>>,
    #[serde(rename = "LicenseUsed")]
    pub license_used: Option<ValueSoap<i64>>,
    #[serde(rename = "LicenseHigh")]
    pub license_high: Option<ValueSoap<i64>>,
}

impl CreateTabledata for BodySystem {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let item = &self.response.result;
        let row: HashMap<ProtoDataFieldId, Data> = fields
            .into_iter()
            .map(|(id, field)| match field.parameter_name.as_str() {
                "Name" => create_string_data(&id, &item.name),
                "System" => create_string_data(&id, &item.system),
                "ConfigFile" => create_string_data(&id, &item.config_file),
                "Directory" => create_string_data(&id, &item.directory),
                "Version" => create_string_data(&id, &item.version),
                "CurrentUsers" => create_int_data(&id, &item.current_users),
                "RoutineCache" => create_int_data(&id, &item.routine_cache),
                "DatabaseCache" => create_int_data(&id, &item.database_cache),
                "LicenseAvailable" => {
                    create_int_data(&id, &item.license_available)
                }
                "LicenseUsed" => create_int_data(&id, &item.license_used),
                "LicenseHigh" => create_int_data(&id, &item.license_high),
                _ => (id.clone(), Err(value::DataError::Missing)),
            })
            .collect();
        vec![row]
    }
}
