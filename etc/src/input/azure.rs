/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt;

use serde::{Serialize,Deserialize};
use schemars::JsonSchema;

use crate::database::DBObj;
use crate::utils::TryAppend;
use crate::specification::*;
use crate::error::Result;
use crate::value::Type;
use super::schema::{MetricValue};

use std::collections::HashMap;

#[derive(Serialize,Deserialize,JsonSchema,Debug,PartialEq,Eq)]
pub struct Input {
	#[serde(rename = "DataTables")]
	pub data_tables: HashMap<DataTableId, ResourceSpec>,
	#[serde(rename = "DataFields")]
	pub data_fields: HashMap<DataFieldId, MetricSpec>,
}


#[derive(Serialize,Deserialize,JsonSchema,Debug,PartialEq,Eq,DBObj)]
pub struct ResourceSpec {
	#[serde(rename = "NameSpace")]
	pub(super) name_space: String,
	#[serde(rename = "DimensionName")]
	pub(super) dimension_name: Option<String>,
}

#[derive(Serialize,Deserialize,JsonSchema,Debug,PartialEq,Eq,DBObj,Clone)]
pub struct MetricSpec {
	#[serde(rename = "MetricName")]
	pub(super) metric_name: String,
	#[serde(rename = "Aggregation")]
	pub(super) aggregation: Aggregation,
	#[serde(rename = "DimensionValue")]
	pub(super) dimension_value: Option<String>,
}

impl MetricSpec {
	pub fn get_type(&self) -> Result<Type> {
		match self.metric_name.as_str() {
			"Resource" | "ResourceGroup" => Ok(Type::String),
			_ => Ok(Type::Float)
		}
	}
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, PartialEq, Eq, Hash, Clone)]
pub enum Aggregation {
	None, Average, Minimum, Maximum, Total, Count
}

impl Aggregation {
	pub fn aggregate_time_series(&self, time_series: &Vec<MetricValue>) -> Option<f64> {
		match self {
			Self::Minimum => time_series.iter().filter_map(|v| v.minimum)
				.fold(None, |a, b| Some(a.map_or(b, |a| a.min(b)))),
			Self::Maximum => time_series.iter().filter_map(|v| v.maximum)
				.fold(None, |a, b| Some(a.map_or(b, |a| a.max(b)))),
			Self::Count => Some(time_series.iter().filter_map(|v| match v.count{
                    Some(x) => Some(x),
                    None => Some(0.0)
            }).fold(0.0, |a, b| a + b)),
			Self::Total => Some(time_series.iter().filter_map(|v| v.total)
				.fold(0.0, |a, b| a + b)),
			Self::Average => {
				let total = time_series.iter().filter_map(|v| v.total)
					.fold(0.0, |a,b| a + b);
				let count = time_series.iter().filter_map(|v| v.count)
					.fold(0.0, |a,b| a + b);
				if count != 0.0 { Some(total / count) } else { None }
			},
			Self::None => None
		}
	}
}

impl fmt::Display for Aggregation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> Result<()> {
		self.data_tables.try_append(other.data_tables)?;
		self.data_fields.try_append(other.data_fields)?;
		Ok(())
    }
}

impl Default for Input {
    fn default() -> Self {
		Input {
	    	data_tables: HashMap::new(),
	    	data_fields: HashMap::new(),
		}
    }
}
