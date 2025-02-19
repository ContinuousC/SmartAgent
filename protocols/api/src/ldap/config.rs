/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

use ldap3::{Ldap, LdapConnAsync, LdapConnSettings, ResultEntry, Scope};
use log::debug;
use native_tls::{Certificate, TlsConnector};
use serde::{Deserialize, Serialize};
use tokio::fs;

use agent_utils::KeyVault;

use crate::{
    get_current_unix_timestamp,
    ldap::{Error, Result},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)]
    pub service_name: Option<String>,
    pub host_config: HostConfig,
    #[serde(default)]
    pub bind_config: Option<BindConfig>,
    #[serde(default)]
    pub search_config: Vec<SearchConfig>,
    #[serde(default)]
    pub replication_config: Option<ReplicationConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostConfig {
    pub hostname: String,
    #[serde(default)]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub port: Option<u16>,
    #[serde(default)]
    pub ssl: bool,
    #[serde(default)]
    pub certificate: Option<(CertificateFormat, PathBuf)>,
    #[serde(default)]
    pub danger_disable_tls_verification: bool,
    #[serde(default)]
    pub danger_disable_hostname_verification: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CertificateFormat {
    Pem,
    Der,
}

impl HostConfig {
    pub fn get_url(&self) -> String {
        format!(
            "ldap{}://{}:{}",
            if self.ssl { "s" } else { "" },
            self.hostname,
            self.port.unwrap_or(if self.ssl { 636 } else { 389 })
        )
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout.unwrap_or(10))
    }

    pub async fn connect(&self) -> Result<(LdapConnAsync, Ldap)> {
        let url = self.get_url();
        debug!("Starting connection with: {}", &url);
        let mut connector = TlsConnector::builder();
        connector
            .danger_accept_invalid_certs(self.danger_disable_tls_verification);
        connector.danger_accept_invalid_hostnames(
            self.danger_disable_hostname_verification,
        );
        if let Some((typ, path)) = &self.certificate {
            let cert = fs::read(path)
                .await
                .map_err(|e| Error::Io(e, path.clone()))?;
            connector.add_root_certificate(match typ {
                CertificateFormat::Pem => Certificate::from_pem(&cert)?,
                CertificateFormat::Der => Certificate::from_der(&cert)?,
            });
        }
        LdapConnAsync::with_settings(
            LdapConnSettings::new()
                .set_connector(connector.build()?)
                .set_conn_timeout(self.timeout()),
            &url,
        )
        .await
        .map_err(Error::Connection)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BindConfig {
    pub bind_user: String,
    #[serde(default)]
    pub bind_pass: String,
}

impl BindConfig {
    pub async fn bind(&self, ldap: &mut Ldap, kvault: &KeyVault) -> Result<()> {
        let (user, pass) =
            match kvault.retrieve_creds(self.bind_user.clone()).await? {
                None => (self.bind_user.clone(), self.bind_pass.clone()),
                Some(creds) => (
                    creds.username.ok_or(Error::InvalidCredential)?,
                    creds.password.ok_or(Error::InvalidCredential)?,
                ),
            };
        debug!("Binding with user: {}", &user);
        ldap.simple_bind(&user, &pass)
            .await
            .map_err(Error::LdapBind)?
            .success()
            .map_err(Error::LdapBind)?;
        Ok(())
    }

    pub async fn unbind(&self, ldap: &mut Ldap) -> Result<()> {
        debug!("unbinding");
        ldap.unbind().await.map_err(Error::LdapUnBind)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ReplicationConfig {
    None,
    All,
    Specific(Vec<String>),
}

impl ReplicationConfig {
    pub fn get_replication_dns(&self) -> Vec<String> {
        match self {
            ReplicationConfig::None => Vec::new(),
            ReplicationConfig::All => {
                vec![String::from("cn=mapping tree,cn=config")]
            }
            ReplicationConfig::Specific(suffixs) => suffixs
                .iter()
                .map(|suffix| {
                    format!(
                        "cn=replica,cn={},cn=mapping tree,cn=config",
                        ldap3::dn_escape(suffix)
                    )
                })
                .collect::<Vec<_>>(),
        }
    }
}

fn default_filter() -> String {
    String::from("(objectclass=*)")
}
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SearchConfig {
    pub base_dn: String,
    pub scope: LdapScope,
    #[serde(default = "default_filter")]
    pub filter: String,
    #[serde(default)]
    pub attributes: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum LdapScope {
    Base,
    OneLevel,
    Subtree,
}

impl SearchConfig {
    pub fn new(
        base_dn: String,
        scope: LdapScope,
        filter: impl ToString,
        attributes: Vec<impl ToString>,
    ) -> Self {
        SearchConfig {
            base_dn,
            scope,
            filter: filter.to_string(),
            attributes: attributes.into_iter().map(|a| a.to_string()).collect(),
        }
    }

    pub async fn search(&self, ldap: &mut Ldap) -> Result<Vec<ResultEntry>> {
        let mut stream = ldap
            .streaming_search(
                &self.base_dn,
                self.scope.to_scope(),
                &self.filter,
                self.attributes.clone(),
            )
            .await
            .map_err(Error::LdapSearch)?;
        let mut results = Vec::with_capacity(1000);

        while let Some(res) = stream.next().await.map_err(Error::LdapSearch)? {
            results.push(res);

            if results.len() == results.capacity() {
                results.reserve(1000);
            }
        }

        Ok(results)
    }

    pub async fn timed_search(
        &self,
        ldap: &mut Ldap,
        timeout: Duration,
    ) -> Result<(usize, Vec<ResultEntry>)> {
        let starttime = get_current_unix_timestamp();

        let res = tokio::time::timeout(timeout, self.search(ldap)).await??;

        let searchtime = get_current_unix_timestamp() - starttime;
        debug!("search took {} μs", &searchtime);
        Ok((get_current_unix_timestamp() - starttime, res))
    }
}

impl LdapScope {
    pub fn to_scope(&self) -> Scope {
        match self {
            LdapScope::Base => Scope::Base,
            LdapScope::OneLevel => Scope::OneLevel,
            LdapScope::Subtree => Scope::Subtree,
        }
    }
}

impl fmt::Display for LdapScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LdapScope::Base => "Base",
                LdapScope::OneLevel => "OneLevel",
                LdapScope::Subtree => "Subtree",
            }
        )
    }
}
