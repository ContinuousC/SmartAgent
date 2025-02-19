/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt;

use serde::{Deserialize, Serialize};

use value::Type;

use super::schema::MetricValue;
use agent_utils::TryAppend;
use etc_base::{DataFieldId, DataTableId};

use std::collections::{HashMap, HashSet};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub struct Input {
    pub data_tables: HashMap<DataTableId, ResourceSpec>,
    pub data_fields: HashMap<DataFieldId, MetricSpec>,
    #[serde(default)] // backwards compatibility with older spec files
    pub data_table_fields: HashMap<DataTableId, HashSet<DataFieldId>>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ResourceSpec {
    pub(super) name_space: String,
    pub(super) dimension_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct MetricSpec {
    pub(super) metric_name: String,
    pub(super) aggregation: Aggregation,
    pub(super) dimension_value: Option<String>,
    pub(super) is_key: bool,
}

impl MetricSpec {
    pub fn get_type(&self) -> Type {
        match self.metric_name.as_str() {
            "Resource" | "ResourceGroup" => Type::UnicodeString,
            _ => Type::Float,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Aggregation {
    None,
    Average,
    Minimum,
    Maximum,
    Total,
    Count,
}

impl Aggregation {
    pub fn aggregate_time_series(
        &self,
        time_series: &Vec<MetricValue>,
    ) -> Option<f64> {
        match self {
            Self::Minimum => time_series
                .iter()
                .filter_map(|v| v.minimum)
                .fold(None, |a, b| Some(a.map_or(b, |a| a.min(b)))),
            Self::Maximum => time_series
                .iter()
                .filter_map(|v| v.maximum)
                .fold(None, |a, b| Some(a.map_or(b, |a| a.max(b)))),
            Self::Count => Some(
                time_series
                    .iter()
                    .filter_map(|v| match v.count {
                        Some(x) => Some(x),
                        None => Some(0.0),
                    })
                    .fold(0.0, |a, b| a + b),
            ),
            Self::Total => Some(
                time_series
                    .iter()
                    .filter_map(|v| v.total)
                    .fold(0.0, |a, b| a + b),
            ),
            Self::Average => {
                let total = time_series
                    .iter()
                    .filter_map(|v| v.total)
                    .fold(0.0, |a, b| a + b);
                let count = time_series
                    .iter()
                    .filter_map(|v| v.count)
                    .fold(0.0, |a, b| a + b);
                if count != 0.0 {
                    Some(total / count)
                } else {
                    None
                }
            }
            Self::None => None,
        }
    }
}

impl fmt::Display for Aggregation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> agent_utils::Result<()> {
        self.data_tables.try_append(other.data_tables)?;
        self.data_fields.try_append(other.data_fields)?;
        self.data_table_fields.try_append(other.data_table_fields)?;
        Ok(())
    }
}
