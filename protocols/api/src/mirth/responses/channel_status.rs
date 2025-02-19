/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use value::{Data, DataError};

use crate::input::FieldSpec;

use super::Value;
#[cfg(feature = "mirth-full")]
use super::{OptValue, Timestamp};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelStatuss {
    #[serde(rename = "dashboardStatus")]
    pub data: Vec<ChannelStatus>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
pub struct ChildChannelStatuss {
    #[serde(rename = "dashboardStatus", default)]
    pub data: Vec<ChannelStatus>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStatus {
    pub channel_id: Value<Uuid>,
    pub name: Value<String>,
    pub state: Value<String>, // chould be an enum?

    #[cfg(feature = "mirth-full")]
    pub queue_enabled: Value<bool>,
    #[cfg(feature = "mirth-full")]
    pub queued: Value<u32>,
    #[cfg(feature = "mirth-full")]
    pub status_type: Value<StatusType>,
    #[cfg(feature = "mirth-full")]
    pub statistics: Statistics,
    #[cfg(feature = "mirth-full")]
    pub lifetime_statistics: Statistics,
    #[cfg(feature = "mirth-full")]
    pub child_statuses: ChildChannelStatuss,

    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub deployed_revision_delta: Value<u32>,
    #[cfg(feature = "mirth-full")]
    #[serde(default)]
    pub deployed_date: Option<Timestamp>,
    #[cfg(feature = "mirth-full")]
    #[serde(rename = "metaDataId", default)]
    // only used in child statusses
    #[cfg(feature = "mirth-full")]
    pub meta_data_id: OptValue<u32>,
    #[cfg(feature = "mirth-full")]
    pub wait_for_previous: Value<bool>,
}

impl ChannelStatus {
    pub fn get_data(&self, field: &FieldSpec) -> Data {
        match field.parameter_header.as_str() {
            "channel_id" => self.channel_id.to_smartm_value(),
            "channel_name" => self.name.to_smartm_value(),
            "state" => self.state.to_smartm_enum(field),
            _ => Err(DataError::Missing),
        }
    }
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Statistics {
    #[serde(rename = "entry")]
    pub data: Vec<serde_json::Value>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
pub struct Statistic {
    pub status: Value<MessageStatus>,
    pub value: Value<u64>,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MessageStatus {
    Received,
    Filtered,
    Sent,
    Error,
}

#[cfg(feature = "mirth-full")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StatusType {
    Channel,
    SourceConnector,
    DestinationConnector,
}
