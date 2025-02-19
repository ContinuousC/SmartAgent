/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use super::{Timestamp, Value};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemStats {
    pub timestamp: Timestamp,
    pub cpu_usage_pct: Value<f64>,
    pub allocated_memory_bytes: Value<u64>,
    pub free_memory_bytes: Value<u64>,
    pub max_memory_bytes: Value<u64>,
    pub disk_free_bytes: Value<u64>,
    pub disk_total_bytes: Value<u64>,
}
