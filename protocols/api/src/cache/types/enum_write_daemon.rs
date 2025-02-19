/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, sync::Arc};

use chrono::{DateTime, Utc};
use etc_base::ProtoDataFieldId;
use protocol::CounterDb;
use serde::Deserialize;
use std::sync::Mutex;
use value::{Data, Value};

use super::generic::{
    create_bool_data, create_data_with_counter_db, create_int_data,
    create_int_enum_data, create_string_data, create_time_data,
    CreateTabledata,
};
use crate::{cache::types::generic::ValueSoap, input::FieldSpec};

mod unix_timestamp {

    use chrono::{DateTime, TimeZone, Utc};
    use serde::{self, Deserialize, Deserializer};

    use crate::cache::types::generic::ValueSoap;

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Option<ValueSoap<DateTime<Utc>>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let i = String::deserialize(deserializer)?;
        let my_int = i
            .split(',')
            .next()
            .map(|s| s.to_string())
            .unwrap_or(i)
            .parse::<i64>()
            .unwrap();
        Utc.timestamp_opt(my_int, 0)
            .single()
            .ok_or(serde::de::Error::invalid_length(
                my_int as usize,
                &"expected a timestamp",
            ))
            .map(|dt| Some(ValueSoap { value: dt }))
    }
}

#[derive(Debug, Deserialize)]
pub struct BodyEnumWriteDaemon {
    #[serde(rename = "EnumWriteDaemonResponse")]
    pub response: EnumWriteDaemonResponse,
}

#[derive(Debug, Deserialize)]
pub struct EnumWriteDaemonResponse {
    #[serde(rename = "EnumWriteDaemonResult")]
    pub result: EnumWriteDaemonResult,
}

#[derive(Debug, Deserialize)]
pub struct EnumWriteDaemonResult {
    #[serde(rename = "diffgram")]
    pub diffgr_diffgram: EnumWriteDaemondiffgram,
}

#[derive(Debug, Deserialize)]
pub struct EnumWriteDaemondiffgram {
    #[serde(rename = "DefaultDataSet")]
    pub data_set: EnumWriteDaemonDataSet,
}

#[derive(Debug, Deserialize)]
pub struct EnumWriteDaemonDataSet {
    #[serde(rename = "Sample")]
    pub samples: Vec<EnumWriteDaemonSample>,
}

#[derive(Debug, Deserialize)]
pub struct EnumWriteDaemonSample {
    #[serde(rename = "Index")]
    pub index: ValueSoap<i64>,
    #[serde(rename = "CurBlk")]
    pub cur_blk: Option<ValueSoap<i64>>,
    #[serde(rename = "TotBlk")]
    pub tot_blk: Option<ValueSoap<u64>>,
    #[serde(rename = "Cycles")]
    pub cycles: Option<ValueSoap<u64>>,
    #[serde(rename = "CycleBlk")]
    pub cycle_blk: Option<ValueSoap<i64>>,
    #[serde(rename = "Wake")]
    pub wake: Option<ValueSoap<bool>>,
    #[serde(rename = "CycleTime")]
    pub cycle_time: Option<ValueSoap<i64>>,
    #[serde(rename = "CycleStart", with = "unix_timestamp")]
    pub cycle_start: Option<ValueSoap<DateTime<Utc>>>,
    #[serde(rename = "Phase")]
    pub phase: Option<ValueSoap<i64>>,
    #[serde(rename = "VolumeQ")]
    pub volume_q: Option<ValueSoap<String>>,
    #[serde(rename = "WakeStart ")]
    pub wake_start: Option<ValueSoap<String>>,
}

impl CreateTabledata for BodyEnumWriteDaemon {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let mut samples_vec: Vec<HashMap<ProtoDataFieldId, Data>> =
            Default::default();
        let list = &self.response.result.diffgr_diffgram.data_set.samples;

        for item in list {
            let row: HashMap<ProtoDataFieldId, Data> = fields
                .iter()
                .map(|(id, field)| match field.parameter_name.as_str() {
                    "Index" => {
                        (id.clone(), Ok(Value::Integer(item.index.value)))
                    }
                    "CurBlk" => create_int_data(id, &item.cur_blk),
                    "TotBlk" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            item.index.value,
                            field.parameter_name.clone()
                        ),
                        &item.tot_blk,
                        &counterdb,
                        field,
                    ),
                    "Cycles" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            item.index.value,
                            field.parameter_name.clone()
                        ),
                        &item.cycles,
                        &counterdb,
                        field,
                    ),
                    "CycleBlk" => create_int_data(id, &item.cycle_blk),
                    "Wake" => create_bool_data(id, &item.wake),
                    "CycleTime" => create_int_data(id, &item.cycle_time),
                    "CycleStart" => create_time_data(id, &item.cycle_start),
                    "Phase" => create_int_enum_data(id, &item.phase, field),
                    "VolumeQ" => create_string_data(id, &item.volume_q),
                    "WakeStart" => create_string_data(id, &item.wake_start),
                    _ => (id.clone(), Err(value::DataError::Missing)),
                })
                .collect();
            samples_vec.push(row);
        }
        samples_vec
    }
}
