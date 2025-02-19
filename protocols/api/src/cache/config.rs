/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{path::PathBuf, time::Duration};

use super::error::{Error, Result};
use agent_utils::KeyVault;
use reqwest::{Certificate, Client};
use serde::{Deserialize, Serialize};
use tokio::fs;
use value::https_port;

use crate::soap::CertType;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HostAllias {
    Domain,
    Ip,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub host: String,
    #[serde(default = "https_port")]
    pub port: Option<u16>,
    pub certificate: Option<(CertType, PathBuf)>,
    #[serde(default)]
    pub disable_certificate_verification: Option<bool>,
    #[serde(default)]
    pub disable_hostname_verification: Option<bool>,
    pub credentials: Credentials,
    pub host_allias: Option<(HostAllias, Option<String>)>,
    pub timeout: Option<u64>,
}

impl Config {
    pub async fn init_client(&self) -> Result<Client> {
        let mut client = Client::builder()
            .danger_accept_invalid_certs(
                self.disable_certificate_verification.unwrap_or(false),
            )
            .danger_accept_invalid_hostnames(
                self.disable_hostname_verification.unwrap_or(false),
            )
            .cookie_store(true) // Necessary
            .timeout(Duration::from_secs(self.timeout.unwrap_or(10)));

        if let Some((cert_type, cert_path)) = self.certificate.as_ref() {
            let cert = fs::read(cert_path).await.map_err(Error::IO)?;
            client =
                client.add_root_certificate(match cert_type {
                    CertType::PEM => Certificate::from_pem(&cert)
                        .map_err(Error::CertParse)?,
                    CertType::DER => Certificate::from_der(&cert)
                        .map_err(Error::CertParse)?,
                });
        }

        Ok(client.build()?)
    }

    pub async fn login(
        &self,
        client: &Client,
        keyvault: &KeyVault,
    ) -> Result<()> {
        // this is needed to place cookies. The WSDL file itself isn't used anywhere.
        let wsdl_url = format!(
            "https://{}:{}/csp/sys/SYS.WSMon.Service.cls?WSDL=1",
            self.get_hostname().await?,
            self.port.unwrap_or(443)
        );

        if self
            .login_with_keyvault(client, keyvault, &wsdl_url)
            .await
            .is_ok()
        {
            log::info!("Login with keyvault succesfull");
            return Ok(());
        }

        let response = client
            .get(wsdl_url)
            .basic_auth(
                &self.credentials.username,
                Some(&self.credentials.password),
            )
            .send()
            .await?;

        if response.status().is_success() {
            log::info!("Login with password succesfull");
            Ok(())
        } else {
            log::error!("Login failed!");
            Err(Error::Authentication)
        }
    }

    pub async fn login_with_keyvault(
        &self,
        client: &Client,
        keyvault: &KeyVault,
        wsdl_url: &str,
    ) -> Result<()> {
        let identity_file = match keyvault {
            KeyVault::Identity => self.credentials.password.clone(),
            KeyVault::KeyReader(_) => {
                keyvault
                    .retrieve_password(self.credentials.username.clone())
                    .await?
            }
        };

        let response = client
            .get(wsdl_url)
            .basic_auth(&self.credentials.username, Some(identity_file))
            .send()
            .await?;

        match response.status().is_success() {
            true => Ok(()),
            false => Err(Error::Authentication),
        }
    }

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

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}
