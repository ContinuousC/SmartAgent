/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use log::debug;
use netsnmp::Oid;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use value::{Data, DataError, Value};

use super::error::Result;
use super::input::ObjectId;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Counters(HashMap<ObjectId, HashMap<Oid, (SystemTime, u64)>>);

impl Counters {
    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        debug!("SNMP: reading counter state");

        match fs::read(path).await {
            Err(e) => debug!("SNMP: failed reading counter state: {}", e),
            Ok(data) => match serde_json::from_slice(&data) {
                Err(e) => debug!("SNMP: counter state undecodable: {}", e),
                Ok(counters) => return Ok(counters),
            },
        }

        debug!("SNMP: using empty counter state");
        Ok(Counters(HashMap::new()))
    }

    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if let Some(dir) = path.as_ref().parent() {
            fs::create_dir_all(&dir).await?;
        }
        let data = serde_json::to_vec(self)?;
        Ok(fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .await?
            .write_all(&data)
            .await?)
    }

    pub fn get_counter(
        &mut self,
        new: u64,
        object: &ObjectId,
        index: &Oid,
    ) -> Data {
        let column = self.0.entry(object.clone()).or_default();
        let now = SystemTime::now();

        let val = match column.get(index) {
            Some((then, old)) => match (new >= *old, now.duration_since(*then))
            {
                (true, Ok(time)) => {
                    Ok(Value::Float((new - *old) as f64 / time.as_secs_f64()))
                }
                _ => Err(DataError::CounterOverflow),
            },
            None => Err(DataError::CounterPending),
        };

        column.insert(index.clone(), (now, new));
        val
    }
}
