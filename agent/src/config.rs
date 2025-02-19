/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use etc_base::{CheckId, Protocol, Tag};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostConfig {
    pub tags: HashSet<Tag>,
    pub checks: HashSet<CheckId>,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(flatten)]
    pub protocols: HashMap<Protocol, Value>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(from = "AgentConfigVx")]
#[serde(into = "AgentConfigVx")]
#[derive(Default)]
pub struct AgentConfig {
    pub write_smartm_data: Option<AgentDataConfig>,
    pub use_password_vault: Option<PasswordVault>,
    #[serde(default = "default_false")]
    pub show_field_errors: bool, // debug only
    #[serde(default = "default_false")]
    pub show_table_info: bool, // debug only
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AgentConfigV2 {
    pub write_smartm_data: Option<AgentDataConfig>,
    pub use_password_vault: Option<PasswordVault>,
    #[serde(default = "default_false")]
    pub show_field_errors: bool, // debug only
    #[serde(default = "default_false")]
    pub show_table_info: bool, // debug only
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AgentConfigV1 {
    #[serde(default = "default_false")]
    pub write_smartm_data: bool,
    #[serde(default = "default_false")]
    pub use_password_vault: bool,
    #[serde(default = "default_false")]
    pub show_field_errors: bool, // debug only
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
enum AgentConfigVx {
    V2(AgentConfigV2),
    V1(AgentConfigV1),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct AgentDataConfig {
    pub instances: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PasswordVault {
    #[serde(rename = "keepass")]
    KeePass,
}

impl Default for AgentDataConfig {
    fn default() -> Self {
        Self {
            instances: vec![String::from("main")],
        }
    }
}

impl Default for PasswordVault {
    fn default() -> Self {
        Self::KeePass
    }
}

/* Backward compatibility: upgrade functions. */

impl From<AgentConfig> for AgentConfigVx {
    fn from(val: AgentConfig) -> Self {
        Self::V2(AgentConfigV2::from(val))
    }
}

impl From<AgentConfigVx> for AgentConfig {
    fn from(val: AgentConfigVx) -> Self {
        Self::from(match val {
            AgentConfigVx::V2(v) => v,
            AgentConfigVx::V1(v) => AgentConfigV2::from(v),
        })
    }
}

impl From<AgentConfig> for AgentConfigV2 {
    fn from(val: AgentConfig) -> Self {
        Self {
            show_field_errors: val.show_field_errors,
            write_smartm_data: val.write_smartm_data,
            use_password_vault: val.use_password_vault,
            show_table_info: val.show_table_info,
        }
    }
}

impl From<AgentConfigV2> for AgentConfig {
    fn from(val: AgentConfigV2) -> Self {
        Self {
            show_field_errors: val.show_field_errors,
            write_smartm_data: val.write_smartm_data,
            use_password_vault: val.use_password_vault,
            show_table_info: val.show_table_info,
        }
    }
}

impl From<AgentConfigV1> for AgentConfigV2 {
    fn from(val: AgentConfigV1) -> Self {
        Self {
            show_table_info: val.show_field_errors,
            show_field_errors: val.show_field_errors,
            write_smartm_data: match val.write_smartm_data {
                true => Some(AgentDataConfig::default()),
                false => None,
            },
            use_password_vault: match val.use_password_vault {
                true => Some(PasswordVault::default()),
                false => None,
            },
        }
    }
}

const fn default_false() -> bool {
    false
}
