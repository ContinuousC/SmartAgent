/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashSet, net::IpAddr};

use netsnmp::Oid;
use serde::{Deserialize, Serialize};

/* Config */

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub host_name: String,
    pub ip_addr: Option<IpAddr>,
    pub host_config: HostConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostConfig {
    pub auth: Option<netsnmp::Auth>,
    #[serde(default = "default_true")]
    pub bulk_host: bool,
    #[serde(default)]
    pub bulk_opts: BulkConfig,
    #[serde(default)]
    pub use_walk: bool,
    #[serde(default)]
    pub quirks: Quirks,
    pub timing: Option<TimingConfig>,
    #[serde(default = "default_workers")]
    pub workers: u16,
    pub port: Option<u16>,
    #[serde(default)]
    pub snmpv3_contexts: Vec<(ContextSelector, HashSet<Option<String>>)>,
}

const fn default_true() -> bool {
    true
}

const fn default_workers() -> u16 {
    1
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct BulkConfig {
    pub max_width: usize,
    pub def_length: usize,
    pub min_length: usize,
    pub max_length: usize,
    pub max_size: usize,
    pub max_len_diff: usize,
}

impl Default for BulkConfig {
    fn default() -> Self {
        BulkConfig {
            max_width: 50,
            def_length: 10,
            min_length: 5,
            max_length: 100,
            max_size: 1000,
            max_len_diff: 5,
        }
    }
}

impl BulkConfig {
    pub(super) fn max_repetitions(
        &self,
        max_expected: Option<usize>,
        available_size: usize,
        walk_width: usize,
    ) -> usize {
        match walk_width {
            0 => 0,
            width => max_expected
                .unwrap_or(self.def_length)
                .max(self.min_length)
                .min(available_size / width)
                .min(self.max_length),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct TimingConfig {
    pub retries: u64,
    pub timeout: f64,
}

impl Default for TimingConfig {
    fn default() -> Self {
        TimingConfig {
            retries: 5,
            timeout: 1.0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ContextSelector {
    All,
    Group(String),
    Oid(Oid),
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
pub struct Quirks {
    /// Continue walk even when OIDs are not increasing.
    pub ignore_oids_not_increasing: bool,
    /// If one of the requested oids would lie past the end of a
    /// table, the device responds with an invalid packet. Seen on
    /// Mitel cluster. To work with such devices in bulk mode, we send
    /// bulk requests until an error is received. Then, we revert to
    /// non-bulk mode. Also we avoid requesting more than one table at
    /// a time in one request, as doing so could force walks on long
    /// tables into non-bulk mode prematurely.
    pub invalid_packets_at_end: bool,
    /// Initiate a new session before each query (i.e. bind another port).
    pub refresh_session: bool,
    /// Number of milliseconds to wait between requests.
    pub request_delay: Option<u64>,
}
