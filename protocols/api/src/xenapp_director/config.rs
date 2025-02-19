/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use protocol::{auth, http};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub http: http::Config,
    pub auth: auth::NtlmAuth,
    pub director_server: Option<String>,
}
