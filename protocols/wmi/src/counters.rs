/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::SystemTime,
};

use lazy_static::lazy_static;
use log::{debug, info, trace};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use value::{DataError, Value};

lazy_static! {
    pub static ref COUNTER_VARIABLES: Vec<String> = vec![
        String::from("Frequency_PerfTime"),
        String::from("Frequency_Sys100NS"),
        String::from("Frequency_Object"),
        String::from("Timestamp_PerfTime"),
        String::from("Timestamp_Sys100NS"),
        String::from("Timestamp_Object")
    ];
    pub static ref REQUIRES_BASE: Vec<WmiCounter> = vec![
        WmiCounter::PerfRawFraction,
        WmiCounter::PerfLargeRawFraction,
        WmiCounter::PerfSampleFraction,
        WmiCounter::PerfAverageBulk,
        WmiCounter::PerfCounterMultiTimer,
        WmiCounter::Perf100nsecMultiTimer,
        WmiCounter::PerfAverageTimer
    ];
}

#[derive(Debug)]
pub struct CounterDB {
    counter_file: PathBuf,
    old_state: HashMap<String, (u64, i64)>,
    new_state: Arc<Mutex<HashMap<String, (u64, i64)>>>,
}

impl CounterDB {
    pub async fn new(counter_file: PathBuf) -> Result<Self, std::io::Error> {
        let old_state: HashMap<String, (u64, i64)> =
            match fs::read(&counter_file).await {
                Err(_) => HashMap::new(),
                Ok(data) => match serde_json::from_reader(data.as_slice()) {
                    Err(_) => HashMap::new(),
                    Ok(ts) => ts,
                },
            };
        Ok(Self {
            counter_file,
            old_state,
            new_state: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    pub fn get(&self, k: &String) -> Option<&(u64, i64)> {
        self.old_state.get(k)
    }
    pub fn insert(&self, k: String, v: (u64, i64)) {
        self.new_state.lock().unwrap().insert(k, v);
    }
    pub async fn save(&self) -> Result<(), std::io::Error> {
        info!("saving counters to {}", self.counter_file.display());
        if let Some(dir) = self.counter_file.parent() {
            fs::create_dir_all(dir).await?;
        }
        let mut f = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&self.counter_file)
            .await?;
        f.write_all(&serde_json::to_vec(&self.new_state)?).await
    }
}

/*
    freq = Frequency
    ts = Timestamp
    time = perftime
    syn = sys100ns
    obj = object
*/
#[allow(dead_code)]
#[derive(Debug)]
pub struct CounterMetadata {
    pub freq_time: u64,
    pub freq_syn: u64,
    pub freq_obj: u64,
    pub ts_time: u64,
    pub ts_syn: u64,
    pub ts_obj: u64,
}

impl CounterMetadata {
    pub fn new(instance: &HashMap<String, String>) -> Result<Self, DataError> {
        // metadata is not available in mssql. so we use current time for this
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        trace!("current timestamp = {now}");

        let mut instance: Vec<u64> = COUNTER_VARIABLES
            .iter()
            .map(|v| {
                instance
                    .get(v)
                    .map(|s| s.parse::<u64>())
                    .unwrap_or_else(|| Ok(match v.contains("Timestamp") {
                        true => now, false => 1
                    }))
                    .map_err(|_e| {
                        DataError::TypeError(format!(
                            "cannot parse {} ({}) to an integer (wmi counter metadata)",
                            v,
                            instance.get(v).unwrap_or(&String::new())
                        ))
                    })
            })
            .collect::<Result<Vec<u64>, DataError>>()?;

        Ok(CounterMetadata {
            ts_obj: instance.pop().unwrap(),
            ts_syn: instance.pop().unwrap(),
            ts_time: instance.pop().unwrap(),
            freq_obj: instance.pop().unwrap(),
            freq_syn: instance.pop().unwrap(),
            freq_time: instance.pop().unwrap(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum WmiCounter {
    // Noncomputational Counter Types (raw values)
    #[serde(rename = "PERF_COUNTER_TEXT", alias = "2816")]
    PerfCounterText,
    #[serde(rename = "PERF_COUNTER_RAWCOUNT", alias = "65536")]
    PerfCounterRawcount,
    #[serde(rename = "PERF_COUNTER_LARGE_RAWCOUNT", alias = "65792")]
    PerfCounterLargeRawcount,
    #[serde(rename = "PERF_COUNTER_RAWCOUNT_HEX", alias = "0")]
    PerfCounterRawcountHex,
    #[serde(rename = "PERF_COUNTER_LARGE_RAWCOUNT_HEX", alias = "256")]
    PerfCounterLargeRawcountHex,

    // Base Counter Types (base values for other counters)
    #[serde(rename = "PERF_AVERAGE_BASE", alias = "1073939458")]
    PerfAverageBase,
    #[serde(rename = "PERF_COUNTER_MULTI_BASE", alias = "1107494144")]
    PerfCounterMultiBase,
    #[serde(rename = "PERF_LARGE_RAW_BASE", alias = "1073939712")]
    PerfLargeRawBase,
    #[serde(rename = "PERF_RAW_BASE", alias = "1073939459")]
    PerfRawBase,
    #[serde(rename = "PERF_SAMPLE_BASE", alias = "1073939457")]
    PerfSampleBase,
    #[serde(rename = "PERF_PRECISION_TIMESTAMP")]
    PerfPrecisionTimestamp,

    // Basic Algorithm Counter Types
    #[serde(rename = "PERF_RAW_FRACTION", alias = "537003008")]
    PerfRawFraction,
    #[serde(rename = "PERF_LARGE_RAW_FRACTION", alias = "537003264")]
    PerfLargeRawFraction,
    #[serde(rename = "PERF_SAMPLE_FRACTION", alias = "549585920")]
    PerfSampleFraction,
    #[serde(rename = "PERF_COUNTER_DELTA", alias = "4195328")]
    PerfCounterDelta,
    #[serde(rename = "PERF_COUNTER_LARGE_DELTA", alias = "4195584")]
    PerfCounterLargeDelta,
    #[serde(rename = "PERF_ELAPSED_TIME", alias = "807666944")]
    PerfElapsedTime,

    // Counter Algorithm Counter Types
    #[serde(rename = "PERF_AVERAGE_BULK", alias = "1073874176")]
    PerfAverageBulk,
    #[serde(rename = "PERF_SAMPLE_COUNTER", alias = "4260864")]
    PerfSampleCounter,
    #[serde(rename = "PERF_COUNTER_COUNTER", alias = "272696320")]
    PerfCounterCounter,
    #[serde(rename = "PERF_COUNTER_BULK_COUNT", alias = "272696576")]
    PerfCounterBulkCount,

    // Precision Timer Algorithm Counter Types
    #[serde(rename = "PERF_PRECISION_TIMER")]
    PerfPrecisionTimer,
    #[serde(rename = "PERF_PRECISION_SYSTEM_TIMER", alias = "541525248")]
    PerfPrecisionSystemTimer,
    #[serde(rename = "PERF_PRECISION_100NS_TIMER", alias = "542573824")]
    PerfPrecision100nsTimer,
    #[serde(rename = "PERF_PRECISION_OBJECT_TIMER")]
    PerfPrecisionObjectTimer,

    // Queue-length Algorithm Counter Types
    #[serde(rename = "PERF_COUNTER_QUEUELEN_TYPE", alias = "4523008")]
    PerfCounterQueuelenType,
    #[serde(rename = "PERF_COUNTER_LARGE_QUEUELEN_TYPE", alias = "4523264")]
    PerfCounterLargeQueuelenType,
    #[serde(rename = "PERF_COUNTER_100NS_QUEUELEN_TYPE", alias = "5571840")]
    PerfCounter100nsQueuelenType,
    #[serde(rename = "PERF_COUNTER_OBJ_TIME_QUEUELEN_TYPE", alias = "6620416")]
    PerfCounterObjTimeQueuelenType,

    // Timer Algorithm Counter Types
    #[serde(rename = "PERF_COUNTER_TIMER", alias = "541132032")]
    PerfCounterTimer,
    #[serde(rename = "PERF_COUNTER_TIMER_INV", alias = "557909248")]
    PerfCounterTimerInv,
    #[serde(rename = "PERF_AVERAGE_TIMER", alias = "805438464")]
    PerfAverageTimer,
    #[serde(rename = "PERF_100NSEC_TIMER", alias = "542180608")]
    Perf100secTimer,
    #[serde(rename = "PERF_100NSEC_TIMER_INV", alias = "558957824")]
    Perf100secTimerInv,

    // Multi-timers ("B represents the number of components being monitored"!?)
    #[serde(rename = "PERF_COUNTER_MULTI_TIMER", alias = "574686464")]
    PerfCounterMultiTimer,
    #[serde(rename = "PERF_COUNTER_MULTI_TIMER_INV", alias = "591463680")]
    PerfCounterMultiTimerInv,
    #[serde(rename = "PERF_100NSEC_MULTI_TIMER", alias = "575735040")]
    Perf100nsecMultiTimer,
    #[serde(rename = "PERF_100NSEC_MULTI_TIMER_INV", alias = "592512256")]
    Perf100nsecMultiTimerInv,

    #[serde(rename = "PERF_OBJ_TIME_TIMER", alias = "543229184")]
    PerfObjTimeTimer,
}

impl WmiCounter {
    pub fn get_wmi_counter(
        &self,
        base_key: &str,
        property: &str,
        counter_db: Arc<CounterDB>,
        instance: &HashMap<String, String>,
    ) -> Result<Value, DataError> {
        let metadata = CounterMetadata::new(instance)?;
        let value = instance.get(property).ok_or(DataError::Missing)?;
        let key = format!("{}_{}", base_key, property);
        trace!("calculating counter for {key} ({value}) of type {self:?} with metadata: {metadata:?}");
        match self {
            // Noncomputational Counter Types (raw values)
            WmiCounter::PerfCounterText => {
                Ok(Value::UnicodeString(value.to_string()))
            }

            // Basic Algorithm Counter Types
            WmiCounter::PerfRawFraction | WmiCounter::PerfLargeRawFraction => {
                Ok(Value::Float(
                    parse_float(&key, value)?
                        / counter_base(property, instance)? as f64,
                ))
            }
            WmiCounter::PerfSampleFraction => get_rate(
                &key,
                parse_int(&key, value)?,
                counter_base(property, instance)? as u64,
                counter_db,
            )
            .map(Value::Float),
            WmiCounter::PerfCounterDelta
            | WmiCounter::PerfCounterLargeDelta => {
                get_delta(&key, parse_int(&key, value)?, counter_db)
                    .map(Value::Integer)
            }
            WmiCounter::PerfElapsedTime => Ok(Value::Float(
                ((metadata.ts_obj - (parse_int(&key, value)? as u64))
                    / metadata.freq_obj) as f64,
            )),

            // Counter Algorithm Counter Types
            WmiCounter::PerfAverageBulk => get_rate(
                &key,
                parse_int(&key, value)?,
                counter_base(property, instance)? as u64,
                counter_db,
            )
            .map(Value::Float),
            WmiCounter::PerfSampleCounter
            | WmiCounter::PerfCounterCounter
            | WmiCounter::PerfCounterBulkCount => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_time / metadata.freq_time,
                counter_db,
            )
            .map(Value::Float),

            // Precision Timer Algorithm Counter Types
            WmiCounter::PerfPrecisionTimer
            | WmiCounter::PerfPrecisionSystemTimer => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_time,
                counter_db,
            )
            .map(Value::Float),
            WmiCounter::PerfPrecision100nsTimer => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_syn,
                counter_db,
            )
            .map(|v| Value::Float(v * 100.0)),
            WmiCounter::PerfPrecisionObjectTimer => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_obj,
                counter_db,
            )
            .map(Value::Float),

            // Queue-length Algorithm Counter Types
            WmiCounter::PerfCounterQueuelenType
            | WmiCounter::PerfCounterLargeQueuelenType => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_time,
                counter_db,
            )
            .map(Value::Float),
            WmiCounter::PerfCounter100nsQueuelenType => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_syn,
                counter_db,
            )
            .map(|v| Value::Float(v * 100.0)),
            WmiCounter::PerfCounterObjTimeQueuelenType => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_obj,
                counter_db,
            )
            .map(Value::Float),

            // Timer Algorithm Counter Types
            WmiCounter::PerfCounterTimer => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_time,
                counter_db,
            )
            .map(Value::Float),
            WmiCounter::PerfCounterTimerInv => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_time,
                counter_db,
            )
            .map(|v| Value::Float(100.0 * (1.0 - v))),
            // https://docs.microsoft.com/en-us/previous-versions/windows/embedded/ms938538(v%3dmsdn.10)
            WmiCounter::PerfAverageTimer =>
            //    Ok(Value::Float(
            //        (get_delta(&key, parse_int(&key, value)?, state)? / (metadata.freq_time as i64)) as f64/
            //        get_delta(&get_base_key(&property), counter_base(&property, instance)?, state)? as f64
            //    )),
            {
                get_rate(
                    &key,
                    (metadata.ts_time / metadata.freq_time) as i64,
                    parse_int(&key, value)? as u64,
                    counter_db,
                )
                .map(Value::Float)
            }
            WmiCounter::Perf100secTimer => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_syn,
                counter_db,
            )
            .map(|v| Value::Float(100.0 * v)),
            WmiCounter::Perf100secTimerInv => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_syn,
                counter_db,
            )
            .map(|v| Value::Float(100.0 * (1.0 - v))),

            WmiCounter::PerfCounterMultiTimer
            | WmiCounter::Perf100nsecMultiTimer => get_rate(
                &key,
                parse_int(&key, value)?,
                counter_base(property, instance)? as u64,
                counter_db,
            )
            .map(|v| Value::Float(100.0 * v)),
            // Multi-timers ("B represents the number of components being monitored"!?)
            // WmiCounter::PerfCounterMultiTimerInv | WmiCounter::Perf100nsecMultiTimerInv => (Type::Float,
            //    (100 * (b - get_rate(key, parse_int(key, value)?, counter_base(property, instance)?, state)))
            //    .map(|v| Box::new(Value::Float(v)))),
            WmiCounter::PerfObjTimeTimer => get_rate(
                &key,
                parse_int(&key, value)?,
                metadata.ts_obj,
                counter_db,
            )
            .map(Value::Float),

            // Noncomputational Counter Types (raw values)
            _ => Ok(Value::Integer(parse_int(&key, value)?)),
        }
    }
}

fn counter_base(
    property: &str,
    instance: &HashMap<String, String>,
) -> Result<i64, DataError> {
    parse_int_from_instance(&format!("{property}_Base"), instance)
        // mssql shenanigans they are not consistent with indicating that a value is the base of another
        .or_else(|_| {
            parse_int_from_instance(&format!("{property} base"), instance)
        })
        .or_else(|_| {
            let mut property_parts =
                property.split_ascii_whitespace().collect::<Vec<_>>();
            property_parts.pop();
            property_parts.push("Base");
            parse_int_from_instance(&property_parts.join(" "), instance)
        })
}

fn parse_int_from_instance(
    key: &str,
    instance: &HashMap<String, String>,
) -> Result<i64, DataError> {
    let value = instance.get(key).ok_or(DataError::Missing)?;
    parse_int(key, value)
}

fn parse_int(key: &str, value: &str) -> Result<i64, DataError> {
    value.parse::<i64>().map_err(|_e| {
        DataError::TypeError(format!(
            "cannot parse {} ({}) to a integer for counter calculcation",
            key, &value
        ))
    })
}

fn parse_float(key: &str, value: &str) -> Result<f64, DataError> {
    value.parse::<f64>().map_err(|_e| {
        DataError::TypeError(format!(
            "cannot parse {} ({}) to a float for counter calculcation",
            &key, value
        ))
    })
}

fn get_delta(
    key: &String,
    value: i64,
    counter_db: Arc<CounterDB>,
) -> Result<i64, DataError> {
    // println!("get_delta: key: {}, value: {}", key, value);
    let res = if let Some((_, last)) = counter_db.get(key) {
        // println!("last_value: {}", last);
        if last > &value {
            Err(DataError::CounterOverflow)
        } else {
            Ok(value - last)
        }
    } else {
        Err(DataError::CounterPending)
    };
    counter_db.insert(key.to_string(), (0, value));
    res
}

fn get_rate(
    key: &String,
    value: i64,
    base: u64,
    counter_db: Arc<CounterDB>,
) -> Result<f64, DataError> {
    debug!("get_rate: key: {key}, value: {value}, base: {base}");
    let res =
        if let Some((last_base, last_value)) = counter_db.get(key).copied() {
            // println!("last_value: {}, last_base: {}", last, last_base);
            if last_value > value || last_base > base {
                Err(DataError::CounterOverflow)
            } else if base == last_base && value == last_value {
                Ok(last_value as f64)
            } else if base == last_base {
                // actually its time diff exeption?
                Err(DataError::CounterUndefined)
            } else {
                // println!("result: {}", (value - last) as f64 / (base - last_base) as f64);
                Ok((value - last_value) as f64 / (base - last_base) as f64)
            }
        } else {
            Err(DataError::CounterPending)
        };
    counter_db.insert(key.to_string(), (base, value));
    res
}
