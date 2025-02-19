/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use std::collections::{HashMap, HashSet};

use etc_base::{CheckId, Protocol, Tag};

#[derive(Serialize, Clone, Debug)]
pub struct HostConfig {
    pub tags: HashSet<Tag>,
    pub checks: HashSet<CheckId>,
    #[serde(default)]
    pub agent: AgentConfig,
    #[serde(flatten)]
    pub protocols: HashMap<Protocol, Box<RawValue>>,
}

// Manually implement Deserialize to avoid bug:
// https://github.com/serde-rs/json/issues/599
impl<'de> Deserialize<'de> for HostConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;
        struct Visitor;

        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = HostConfig;

            fn expecting(
                &self,
                formatter: &mut std::fmt::Formatter,
            ) -> std::fmt::Result {
                write!(formatter, "a host configuration object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let mut tags: Option<HashSet<Tag>> = None;
                let mut checks: Option<HashSet<CheckId>> = None;
                let mut agent: Option<AgentConfig> = None;
                let mut protocols: HashMap<Protocol, Box<RawValue>> =
                    HashMap::new();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "tags" => {
                            tags = Some(map.next_value()?);
                        }
                        "checks" => {
                            checks = Some(map.next_value()?);
                        }
                        "agent" => {
                            agent = Some(map.next_value()?);
                        }
                        _ => {
                            protocols.insert(Protocol(key), map.next_value()?);
                        }
                    }
                }

                Ok(HostConfig {
                    tags: tags.ok_or_else(|| {
                        A::Error::custom("missing field 'tags'")
                    })?,
                    checks: checks.ok_or_else(|| {
                        A::Error::custom("missing field 'checks'")
                    })?,
                    agent: agent.ok_or_else(|| {
                        A::Error::custom("missing field 'agent'")
                    })?,
                    protocols,
                })
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(from = "AgentConfigVx")]
#[serde(into = "AgentConfigVx")]
pub struct AgentConfig {
    pub write_smartm_data: Option<AgentDataConfig>,
    pub use_password_vault: Option<PasswordVault>,
    #[serde(default)]
    pub error_reporting: ErrorReporting,
    #[serde(default)]
    pub run_noninventorized_checks: bool,
    #[serde(default)]
    pub show_field_errors: bool, // debug only
    #[serde(default)]
    pub show_table_info: bool, // debug only
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AgentConfigV2 {
    pub write_smartm_data: Option<AgentDataConfig>,
    pub use_password_vault: Option<PasswordVault>,
    #[serde(default)]
    pub error_reporting: ErrorReporting,
    #[serde(default)]
    pub run_noninventorized_checks: bool,
    #[serde(default)]
    pub show_field_errors: bool, // debug only
    #[serde(default)]
    pub show_table_info: bool, // debug only
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AgentConfigV1 {
    #[serde(default)]
    pub write_smartm_data: bool,
    #[serde(default)]
    pub use_password_vault: bool,
    #[serde(default)]
    pub show_field_errors: bool, // debug only
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ErrorReporting {
    Handle {
        #[serde(default)]
        move_error_file: bool,
    },
    Legacy,
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

impl Default for ErrorReporting {
    fn default() -> Self {
        Self::Handle {
            move_error_file: false,
        }
    }
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
            run_noninventorized_checks: val.run_noninventorized_checks,
            error_reporting: val.error_reporting,
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
            run_noninventorized_checks: val.run_noninventorized_checks,
            error_reporting: val.error_reporting,
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
            run_noninventorized_checks: false,
            error_reporting: ErrorReporting::default(),
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
