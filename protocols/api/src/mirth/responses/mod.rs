/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{fmt::Display, ops::Deref, sync::Arc, time::SystemTime};

use protocol::CounterDb;
use serde::{Deserialize, Serialize};

mod channel_group;
mod channel_specific;
mod channel_statistics;
mod channel_status;
mod system_info;

#[cfg(feature = "mirth-full")]
mod server_status;
#[cfg(feature = "mirth-full")]
pub use server_status::ServerStatus;
#[cfg(feature = "mirth-full")]
mod system_stats;
#[cfg(feature = "mirth-full")]
pub use system_stats::SystemStats;

pub use channel_group::ChannelGroups;
pub use channel_specific::{ChannelConnector, ChannelSpecific, ConnectorType};
pub use channel_statistics::ChannelStatistics;
pub use channel_status::ChannelStatuss;
pub use system_info::SystemInfo;

#[cfg(feature = "mirth-full")]
pub use system_stats::SystemStats;
use uuid::Uuid;
use value::{Data, EnumValue};

use crate::input::{FieldSpec, ParameterType, ValueTypes};

#[derive(Debug, Serialize, Deserialize)]
pub struct Value<T> {
    #[serde(rename = "$value")]
    pub data: T,
}

impl<T: Default> Default for Value<T> {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl<T> Deref for Value<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: Display> Display for Value<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.data.fmt(f)
    }
}

impl Value<String> {
    pub fn to_smartm_value(&self) -> value::Data {
        Ok(value::Value::UnicodeString(self.data.clone()))
    }
    pub fn to_smartm_enum(&self, field: &FieldSpec) -> value::Data {
        if let Some(ValueTypes::String(vals)) = &field.values {
            EnumValue::new(vals.clone(), self.data.clone())
                .map(value::Value::Enum)
        } else {
            Err(value::DataError::External(
                "expected a string enum".to_string(),
            ))
        }
    }
}
impl Value<Uuid> {
    pub fn to_smartm_value(&self) -> value::Data {
        Ok(value::Value::UnicodeString(self.data.to_string()))
    }
}
impl Value<u64> {
    pub fn to_smartm_value(&self) -> value::Data {
        Ok(value::Value::Integer(self.data as i64))
    }
    pub fn to_smartm_counter(
        &self,
        channel_id: Uuid,
        field: &FieldSpec,
        counter_db: Arc<CounterDb>,
    ) -> Data {
        let key = format!("{}.{}", channel_id, field.parameter_name);
        let now = SystemTime::now();
        match field.parameter_type {
            ParameterType::Counter => counter_db.counter(key, self.data, now),
            ParameterType::Difference => {
                counter_db.difference(key, self.data, now)
            }
            _ => self.to_smartm_value(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptValue<T> {
    #[serde(default, rename = "$value")]
    pub data: Option<T>,
}

impl<T: Default> Default for OptValue<T> {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

impl<T> Deref for OptValue<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: Display> Display for OptValue<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            None => write!(f, "None"),
            Some(t) => t.fmt(f),
        }
    }
}

impl OptValue<String> {
    pub fn to_smartm_value(&self) -> value::Data {
        self.data
            .as_ref()
            .map(|s| value::Value::UnicodeString(s.to_string()))
            .ok_or(value::DataError::Missing)
    }
}
#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timestamp {
    pub time: Value<u64>,
    pub timezone: String,
}
