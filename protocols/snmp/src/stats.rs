/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::hash_map;
use std::collections::HashMap;
use std::path::Path;

use netsnmp::Oid;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use log::debug;

use super::error::Result;
use super::walk::WalkStats;

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub struct Stats(HashMap<Oid, WalkStats>);

impl Stats {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        debug!("SNMP: reading walk statistics");
        match fs::read(path).await {
            Err(e) => debug!("SNMP: failed reading walk statistics: {}", e),
            Ok(data) => match serde_json::from_reader(data.as_slice()) {
                Err(e) => debug!("SNMP: walk statistics undecodable: {}", e),
                Ok(stats) => return Ok(stats),
            },
        }

        debug!("SNMP: using empty statistics");
        Ok(Stats(HashMap::new()))
    }

    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        if let Some(dir) = path.as_ref().parent() {
            fs::create_dir_all(dir).await?;
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

    pub fn walk_entry<'a: 'b, 'b>(
        &'a mut self,
        oid: Oid,
    ) -> hash_map::Entry<'b, Oid, WalkStats> {
        self.0.entry(oid)
    }

    pub fn get_walk(&self, oid: &Oid) -> Option<&WalkStats> {
        self.0.get(oid)
    }
}
