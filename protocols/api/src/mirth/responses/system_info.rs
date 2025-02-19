/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};
use value::{Data, DataError};

use crate::input::FieldSpec;

use super::Value;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub jvm_version: Value<String>,
    pub os_name: Value<String>,
    pub os_version: Value<String>,
    pub os_architecture: Value<String>,
    pub db_name: Value<String>,
    pub db_version: Value<String>,
}

impl SystemInfo {
    pub fn get_data(&self, field: &FieldSpec) -> Data {
        match field.parameter_header.as_str() {
            "jvm_version" => self.jvm_version.to_smartm_value(),
            "os_name" => self.os_name.to_smartm_value(),
            "os_version" => self.os_version.to_smartm_value(),
            "os_architecture" => self.os_architecture.to_smartm_value(),
            "db_name" => self.db_name.to_smartm_value(),
            "db_version" => self.db_version.to_smartm_value(),
            _ => Err(DataError::Missing),
        }
    }
}
