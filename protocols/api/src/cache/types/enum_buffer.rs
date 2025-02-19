/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, sync::Arc};

use crate::input::FieldSpec;

use super::generic::{
    create_data_with_counter_db, create_int_data, CreateTabledata, ValueSoap,
};
use etc_base::ProtoDataFieldId;
use protocol::CounterDb;
use serde::Deserialize;
use std::sync::Mutex;
use value::{Data, Value};

#[derive(Debug, Deserialize)]
pub struct BodyEnumBuffer {
    #[serde(rename = "EnumBufferResponse")]
    pub response: EnumBufferResponse,
}

#[derive(Debug, Deserialize)]
pub struct EnumBufferResponse {
    #[serde(rename = "EnumBufferResult")]
    pub result: EnumBufferResult,
}

#[derive(Debug, Deserialize)]
pub struct EnumBufferResult {
    #[serde(rename = "diffgram")]
    pub diffgr_diffgram: EnumBufferdiffgram,
}

#[derive(Debug, Deserialize)]
pub struct EnumBufferdiffgram {
    #[serde(rename = "DefaultDataSet")]
    pub data_set: EnumBufferDefaultDataSet,
}

#[derive(Debug, Deserialize)]
pub struct EnumBufferDefaultDataSet {
    #[serde(rename = "Sample")]
    pub samples: Vec<EnumBufferSample>,
}

#[derive(Debug, Deserialize)]
pub struct EnumBufferSample {
    #[serde(rename = "Size")]
    pub size: ValueSoap<i64>, // Size is not made optional as it is the identifier for a buffer, we can be certain it will always be in the API response
    #[serde(rename = "NumSize")]
    pub num_size: Option<ValueSoap<i64>>,
    #[serde(rename = "BatchQ")]
    pub batch_q: Option<ValueSoap<i64>>,
    #[serde(rename = "Interact")]
    pub interact: Option<ValueSoap<i64>>,
    #[serde(rename = "MaxInteract")]
    pub max_interact: Option<ValueSoap<i64>>,
    #[serde(rename = "MinReQ")]
    pub min_re_q: Option<ValueSoap<i64>>,
    #[serde(rename = "MinReQB")]
    pub min_re_qb: Option<ValueSoap<i64>>,
    #[serde(rename = "ReQCnt")]
    pub re_qcnt: Option<ValueSoap<u64>>,
    #[serde(rename = "ReQCntB")]
    pub re_qcnt_b: Option<ValueSoap<u64>>,
    #[serde(rename = "WrtQSz")]
    pub wrt_qsz: Option<ValueSoap<i64>>,
    #[serde(rename = "OffLRUCnt")]
    pub off_lrucnt: Option<ValueSoap<u64>>,
    #[serde(rename = "WrtSz")]
    pub wrt_sz: Option<ValueSoap<i64>>,
    #[serde(rename = "WrtMax")]
    pub wrt_max: Option<ValueSoap<i64>>,
    #[serde(rename = "Avail")]
    pub avail: Option<ValueSoap<i64>>,
    #[serde(rename = "Min")]
    pub min: Option<ValueSoap<i64>>,
    #[serde(rename = "MinB")]
    pub min_b: Option<ValueSoap<i64>>,
}
impl CreateTabledata for BodyEnumBuffer {
    fn create_tabledata(
        self,
        fields: HashMap<ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<Mutex<CounterDb>>,
    ) -> Vec<HashMap<ProtoDataFieldId, Data>> {
        let samples = self.response.result.diffgr_diffgram.data_set.samples;

        let mut samples_vec: Vec<HashMap<ProtoDataFieldId, Data>> =
            Vec::with_capacity(samples.len());
        for sample in &samples {
            let row: HashMap<ProtoDataFieldId, Data> = fields
                .iter()
                .map(|(id, field)| match field.parameter_name.as_str() {
                    "Size" => {
                        (id.clone(), Ok(Value::Integer(sample.size.value)))
                    }
                    "NumSize" => create_int_data(id, &sample.num_size),
                    "BatchQ" => create_int_data(id, &sample.batch_q),
                    "Interact" => create_int_data(id, &sample.interact),
                    "MaxInteract" => create_int_data(id, &sample.max_interact),
                    "MinReQ" => create_int_data(id, &sample.min_re_q),
                    "MinReQB" => create_int_data(id, &sample.min_re_qb),
                    "ReQCnt" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            sample.size.value,
                            field.parameter_name.clone()
                        ),
                        &sample.re_qcnt,
                        &counterdb,
                        field,
                    ),
                    "ReQCntB" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            sample.size.value,
                            field.parameter_name.clone()
                        ),
                        &sample.re_qcnt_b,
                        &counterdb,
                        field,
                    ),
                    "WrtQSz" => create_int_data(id, &sample.wrt_qsz),
                    "OffLRUCnt" => create_data_with_counter_db(
                        id,
                        format!(
                            "{}.{}",
                            sample.size.value,
                            field.parameter_name.clone()
                        ),
                        &sample.off_lrucnt,
                        &counterdb,
                        field,
                    ),
                    "WrtSz" => create_int_data(id, &sample.wrt_sz),
                    "WrtMax" => create_int_data(id, &sample.wrt_max),
                    "Avail" => create_int_data(id, &sample.avail),
                    "Min" => create_int_data(id, &sample.min),
                    "MinB" => create_int_data(id, &sample.min_b),
                    _ => (id.clone(), Err(value::DataError::Missing)),
                })
                .collect();
            samples_vec.push(row);
        }
        samples_vec
    }
}
