/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use agent_utils::KeyVault;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)]
    pub vmware: Option<super::vmware::Config>,
    #[serde(default)]
    pub ms_graph: Option<super::ms_graph::Config>,
    #[serde(default)]
    pub azure: Option<azure_protocol::Config>,
    #[serde(default)]
    pub ldap: Option<Vec<super::ldap::Config>>,
    #[serde(default)]
    pub cache: Option<super::cache::Config>,
    #[serde(default)]
    pub mirth: Option<super::mirth::Config>,
    #[serde(default)]
    pub unity: Option<super::unity::Config>,
    #[serde(default)]
    pub xenapp_director: Option<super::xenapp_director::Config>,
    #[serde(default)]
    pub proxmox: Option<super::proxmox::Config>,
    #[serde(default)]
    pub elastic: Option<super::elastic::Config>, //external: HashMap<PluginId,Value>,
}

pub type KeyvaultResult<T> = std::result::Result<T, KeyvaultError>;

#[derive(Debug, thiserror::Error)]
pub enum KeyvaultError {
    #[error("Entry not found in keyvault")]
    MissingKREntry,
    #[error("No {0} in KeyVault entry")]
    MissingKRObject(String),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
}

pub async fn from_keyvault(
    key_vault: &KeyVault,
    entry: String,
    default: &str,
) -> KeyvaultResult<(String, String)> {
    match key_vault {
        KeyVault::Identity => Ok((entry, default.to_string())),
        _ => {
            let kr_entry = key_vault
                .retrieve_creds(entry)
                .await?
                .ok_or(KeyvaultError::MissingKREntry)?;
            let username = kr_entry
                .username
                .as_ref()
                .ok_or(KeyvaultError::MissingKRObject(String::from(
                    "username",
                )))?
                .split('@')
                .next()
                .ok_or(KeyvaultError::MissingKRObject(String::from(
                    "username",
                )))?
                .to_string();
            let password = kr_entry
                .password
                .as_ref()
                .ok_or(KeyvaultError::MissingKRObject(String::from(
                    "password",
                )))?
                .to_string();
            Ok((username, password))
        }
    }
}
