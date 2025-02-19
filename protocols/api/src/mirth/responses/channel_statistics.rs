/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::sync::Arc;

use protocol::CounterDb;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use value::{Data, DataError};

use crate::input::FieldSpec;

use super::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelStatistics {
    #[serde(rename = "channelStatistics")]
    pub data: Vec<ChannelStatistic>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelStatistic {
    #[cfg(feature = "mirth-full")]
    pub server_id: Value<Uuid>,
    pub channel_id: Value<Uuid>,
    pub received: Value<u64>,
    pub sent: Value<u64>,
    pub error: Value<u64>,
    pub filtered: Value<u64>,
    pub queued: Value<u64>,
}

impl ChannelStatistic {
    pub fn get_data(
        &self,
        field: &FieldSpec,
        counterdb: Arc<CounterDb>,
    ) -> Data {
        match field.parameter_header.as_str() {
            "channel_id" => self.channel_id.to_smartm_value(),
            "received" => self.received.to_smartm_counter(
                self.channel_id.data,
                field,
                counterdb,
            ),
            "sent" => self.sent.to_smartm_counter(
                self.channel_id.data,
                field,
                counterdb,
            ),
            "error" => self.error.to_smartm_counter(
                self.channel_id.data,
                field,
                counterdb,
            ),
            "filtered" => self.filtered.to_smartm_counter(
                self.channel_id.data,
                field,
                counterdb,
            ),
            "queued" => self.queued.to_smartm_counter(
                self.channel_id.data,
                field,
                counterdb,
            ),
            _ => Err(DataError::Missing),
        }
    }
}
