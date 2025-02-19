/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use etc_base::{ProtoDataFieldId, ProtoRow};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use value::DataError;

use crate::input::FieldSpec;

use super::Value;
#[cfg(feature = "mirth-full")]
use super::{OptValue, Timestamp};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChannelGroups {
    #[serde(rename = "channelGroup")]
    pub data: Vec<ChannelGroup>,
}

impl ChannelGroups {
    pub fn get_channels(&self) -> Vec<Uuid> {
        self.data
            .iter()
            .flat_map(|cg| cg.channels.data.iter().map(|ch| ch.id.data))
            .collect()
    }
    pub fn get_data<'a>(
        &self,
        fields: HashMap<&'a ProtoDataFieldId, &'a FieldSpec>,
    ) -> Vec<ProtoRow> {
        self.data
            .iter()
            .flat_map(|cg| {
                cg.channels.data.iter().map(|ch| {
                    fields
                        .iter()
                        .map(|(fid, f)| {
                            (
                                (*fid).clone(),
                                match f.parameter_header.as_str() {
                                    "group_id" => cg.id.to_smartm_value(),
                                    "group_name" => cg.name.to_smartm_value(),
                                    "channel_id" => ch.id.to_smartm_value(),
                                    _ => Err(DataError::Missing),
                                },
                            )
                        })
                        .collect()
                })
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelGroup {
    pub id: Value<Uuid>,
    pub name: Value<String>,
    pub channels: Channels,
    #[cfg(feature = "mirth-full")]
    pub revision: Value<u32>,
    #[cfg(feature = "mirth-full")]
    pub last_modified: Timestamp,
    #[cfg(feature = "mirth-full")]
    pub description: OptValue<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Channels {
    #[serde(rename = "channel", default)]
    pub data: Vec<Channel>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Channel {
    pub id: Value<Uuid>,
    #[cfg(feature = "mirth-full")]
    pub revision: Value<u32>,
}
