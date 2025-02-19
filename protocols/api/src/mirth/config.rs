/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use protocol::{auth, http};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub http_client: http::Config,
    pub api_auth: auth::BasicAuth,
    #[serde(default)]
    pub smb_auth: Option<auth::NtlmAuth>,
    #[serde(default)]
    pub smb_opts: Option<SmbOpts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmbOpts {
    #[serde(default = "max_concurrent")]
    pub max_concurrent: usize,
    #[serde(default)]
    pub server_mapping: HashMap<String, String>,
}

fn max_concurrent() -> usize {
    20
}
