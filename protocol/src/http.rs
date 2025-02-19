/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    path::{Path, PathBuf},
    sync::Arc,
};

use agent_utils::ip_lookup_one;
use reqwest::{
    cookie::Jar,
    header::{HeaderName, HeaderValue},
};
use reqwest::{Certificate, Client};
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    IpLookup(#[source] agent_utils::Error),

    #[error("unable to open file {0}: {1}")]
    ReadFile(PathBuf, #[source] std::io::Error),
    #[error("unable to parse the provided certificate {0}: {1}")]
    ParseCertificate(PathBuf, #[source] reqwest::Error),
    #[error("unable to build a http client: {0}")]
    BuildApiClient(#[source] reqwest::Error),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HostAlias {
    Domain(String),
    Ip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub hostname: String,
    pub port: Option<u16>,
    #[serde(default)]
    pub ipaddress: Option<Ipv4Addr>,
    #[serde(default)]
    pub https_strategy: HttpsStrategy,
    pub host_allias: Option<HostAlias>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpsStrategy {
    #[default]
    Strict,
    Specific(PathBuf),
    IgnoreHostname(Option<PathBuf>),
    IgnoreCertificate,
    Http,
}

impl Config {
    pub async fn create_client(
        &self,
        headers: Vec<(HeaderName, HeaderValue)>,
    ) -> Result<(Client, Arc<Jar>)> {
        let cookiejar = Arc::new(Jar::default());
        let mut builder = reqwest::Client::builder()
            .cookie_provider(cookiejar.clone()) // by some protocols
            .user_agent("SmartAgent")
            .danger_accept_invalid_hostnames(matches!(
                self.https_strategy,
                HttpsStrategy::IgnoreHostname(_)
            ))
            .danger_accept_invalid_certs(matches!(
                self.https_strategy,
                HttpsStrategy::IgnoreCertificate
            ))
            .default_headers(headers.into_iter().collect());

        if let Some(ip) = self.ipaddress {
            builder = builder.resolve(
                &self.true_hostname().await?,
                SocketAddr::V4(SocketAddrV4::new(ip, 0)),
            );
        }

        if let HttpsStrategy::Specific(path) = &self.https_strategy {
            let certificate = Self::load_certificate(path).await?;
            builder = builder
                .add_root_certificate(certificate)
                .tls_built_in_root_certs(false);
        }
        if let HttpsStrategy::IgnoreHostname(Some(path)) = &self.https_strategy
        {
            let certificate = Self::load_certificate(path).await?;
            builder = builder
                .add_root_certificate(certificate)
                .tls_built_in_root_certs(false);
        }

        builder
            .build()
            .map(|c| (c, cookiejar))
            .map_err(Error::BuildApiClient)
    }

    async fn load_certificate(path: &Path) -> Result<Certificate> {
        let content = tokio::fs::read(&path)
            .await
            .map_err(|e| Error::ReadFile(path.to_path_buf(), e))?;
        Certificate::from_der(&content)
            .or_else(|_| Certificate::from_pem(&content))
            .map_err(|e| Error::ParseCertificate(path.to_path_buf(), e))
    }

    pub async fn base_url(&self, default_port: Option<u16>) -> Result<String> {
        Ok(format!(
            "{}://{}:{}",
            self.scheme(),
            self.true_hostname().await?,
            default_port.unwrap_or(self.http_port())
        ))
    }

    pub fn scheme(&self) -> &'static str {
        matches!(self.https_strategy, HttpsStrategy::Http)
            .then_some("http")
            .unwrap_or("https")
    }

    pub async fn true_hostname(&self) -> Result<String> {
        Self::alias_host(
            self.host_allias.as_ref(),
            &self.hostname,
            self.ipaddress.as_ref(),
        )
        .await
    }
    pub async fn alias_host(
        host_alias: Option<&HostAlias>,
        hostname: &str,
        ipaddress: Option<&Ipv4Addr>,
    ) -> Result<String> {
        match host_alias {
            None => Ok(hostname.to_string()),
            Some(HostAlias::Domain(d)) => Ok(format!("{}.{}", &hostname, d)),
            Some(HostAlias::Ip) => {
                if let Some(ip) = ipaddress {
                    Ok(ip.to_string())
                } else {
                    ip_lookup_one(hostname)
                        .await
                        .map(|ip| ip.to_string())
                        .map_err(Error::IpLookup)
                }
            }
        }
    }

    pub fn http_port(&self) -> u16 {
        self.port.unwrap_or_else(|| {
            matches!(self.https_strategy, HttpsStrategy::Http)
                .then_some(80)
                .unwrap_or(443)
        })
    }
}
