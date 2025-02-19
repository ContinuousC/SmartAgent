/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use rule_engine::{config::Config, secret::Secret};

#[derive(Serialize, Deserialize, Config, Debug)]
pub struct EssentialConfig {
    #[config(title = "VMWare API Port")]
    pub port: Option<u16>,
    #[config(title = "Certificate")]
    pub certificate: Option<CertType>,
    #[config(title = "Credentials")]
    pub credentials: Credentials,
    #[config(title = "Is a cluster", default = "default_false")]
    pub is_cluster: bool,
    #[config(title = "Host alias")]
    pub host_alias: Option<HostAlias>,
    #[config(
        title = "Disable certificate verification (dangerous!)",
        default = "default_false"
    )]
    pub disable_certificate_verification: bool,
    #[config(
        title = "Disable hostname verification (dangerous!)",
        default = "default_false"
    )]
    pub disable_hostname_verification: bool,
}

const fn default_false() -> bool {
    false
}

#[derive(Serialize, Deserialize, Config, Clone, Debug)]
pub struct Credentials {
    #[config(title = "Username")]
    pub username: String,
    #[config(title = "Password")]
    pub password: Option<Secret>,
}

#[derive(Serialize, Deserialize, Config, Clone, Debug)]
pub enum HostAlias {
    Domain(String),
    Ip(Option<String>),
}

#[derive(Serialize, Deserialize, Config, Clone, Debug)]
pub enum CertType {
    PEM(String),
    DER(String),
}

impl EssentialConfig {
    pub(crate) fn into_omd_config(self, host: String) -> super::config::Config {
        super::config::Config {
            host,
            port: self.port,
            certificate: match self.certificate {
                Some(CertType::PEM(path)) => {
                    Some((crate::soap::CertType::PEM, PathBuf::from(path)))
                }
                Some(CertType::DER(path)) => {
                    Some((crate::soap::CertType::DER, PathBuf::from(path)))
                }
                None => None,
            },
            credentials: Some(super::config::Credentials {
                username: self.credentials.username,
                password: self.credentials.password.map(|p| {
                    p.secret.map_or_else(String::new, |s| {
                        String::from_utf8_lossy(s.unsecure()).to_string()
                    })
                }),
            }),
            host_allias: match self.host_alias {
                Some(HostAlias::Domain(domain)) => {
                    Some((super::config::HostAllias::Domain, Some(domain)))
                }
                Some(HostAlias::Ip(ip)) => {
                    ip.map(|ip| (super::config::HostAllias::Ip, Some(ip)))
                }
                None => None,
            },
            is_cluster: Some(self.is_cluster),
            disable_certificate_verification: Some(
                self.disable_certificate_verification,
            ),
            disable_hostname_verification: Some(
                self.disable_hostname_verification,
            ),
        }
    }
}
