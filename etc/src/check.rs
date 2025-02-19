/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use agent_utils::DBObj;
use etc_base::{CheckId, FieldId, MPId, TableId};

#[derive(Serialize, Deserialize, Clone, DBObj, Debug, PartialEq, Eq)]
pub struct CheckSpec {
    #[serde(rename = "MP")]
    pub mp: MPId,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "Tables")]
    pub tables: Vec<TableId>,
    #[serde(rename = "Parent")]
    pub parent: Option<CheckId>,
    #[serde(default, rename = "DefaultGrouping")]
    pub default_grouping: DefaultGrouping,
}

#[derive(
    smart_default::SmartDefault,
    Serialize,
    Deserialize,
    Clone,
    DBObj,
    Debug,
    PartialEq,
    Eq,
)]
pub enum DefaultGrouping {
    #[default]
    #[serde(rename = "default")]
    Default,
    #[serde(rename = "single")]
    Single,
    #[serde(rename = "groupByOrDefault")]
    GroupByOrDefault(Vec<FieldId>),
    #[serde(rename = "groupByOrExclude")]
    GroupByOrExclude(Vec<FieldId>),
}
