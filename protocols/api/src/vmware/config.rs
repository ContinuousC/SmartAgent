/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::soap::CertType;
use crate::vmware::error::Result;
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: Option<u16>,
    pub certificate: Option<(CertType, PathBuf)>,
    pub credentials: Option<Credentials>,
    pub is_cluster: Option<bool>,
    pub host_allias: Option<(HostAllias, Option<String>)>,
    pub disable_certificate_verification: Option<bool>,
    pub disable_hostname_verification: Option<bool>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Credentials {
    pub username: String,
    pub password: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HostAllias {
    Domain,
    Ip,
}

impl Config {
    pub async fn get_hostname(&self) -> Result<String> {
        match &self.host_allias {
            Some((HostAllias::Domain, Some(domain))) => {
                Ok(format!("{}.{}", self.host, domain))
            }
            Some((HostAllias::Ip, Some(ip))) => Ok(ip.clone()),
            Some((HostAllias::Ip, None)) => {
                Ok(agent_utils::ip_lookup_one(&self.host).await?.to_string())
            }
            _ => Ok(self.host.clone()),
        }
    }
}
