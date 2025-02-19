/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::HashMap, fmt::Display, net::IpAddr, path::PathBuf, sync::Arc,
};

use agent_utils::KeyVault;
use serde::{Deserialize, Serialize};
use tap::Pipe;

use crate::{error::Result, sqlplugin::SqlPlugin, ConnectionString};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub hostname: String,
    #[serde(default)]
    /// used to resolve mssql instances
    /// if it is not set, use dns
    pub ip: Option<IpAddr>,
    #[serde(default)]
    pub instances: Vec<InstanceType>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub driver: Option<String>,
    #[serde(default)]
    pub database: Option<String>,
    #[serde(default)]
    pub timeout: Option<u16>,
    #[serde(default)]
    pub ssl: Option<SslMode>,
    #[serde(default)]
    pub ssl_key: Option<PathBuf>,
    #[serde(default)]
    pub ssl_cert: Option<PathBuf>,
    #[serde(default)]
    pub disable_certificate_verification: Option<bool>,
    #[serde(default)]
    pub encrypt: Option<bool>,
    #[serde(default)]
    pub dsn: Option<String>,
    #[serde(default)]
    pub file_dsn: Option<PathBuf>,
    #[serde(default)]
    pub custom_args: Vec<(String, String)>,
    #[serde(default)]
    pub connection_string: Option<String>,
}

impl Config {
    pub async fn generic_connectionstring(
        self: Arc<Self>,
        sql_plugin: Arc<dyn SqlPlugin>,
        kvault: &KeyVault,
    ) -> Result<HashMap<InstanceType, String>> {
        if let Some(cs) = self.connection_string.as_ref() {
            return Ok(self
                .instances
                .iter()
                .map(|inst| (inst.clone(), cs.clone()))
                .collect());
        }

        sql_plugin
            .connection_string_per_instance(
                ConnectionString::from_config(self.clone(), kvault).await?,
                self,
            )
            .await?
            .into_iter()
            .map(|(inst, cs)| (inst, cs.to_string()))
            .collect::<HashMap<InstanceType, String>>()
            .pipe(Ok)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SslMode {
    Require,
    Prefer,
    Allow,
    Disable,
}

impl Display for SslMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Require => "require",
                Self::Prefer => "prefer",
                Self::Allow => "allow",
                Self::Disable => "disable",
            }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
#[serde(untagged)]
pub enum InstanceType {
    String(String),
    Port(u16),
    Default,
}

impl Display for InstanceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::String(s) => s.clone(),
                Self::Port(p) => p.to_string(),
                Self::Default => "Default".to_string(),
            }
        )
    }
}
