/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, net::IpAddr, path::PathBuf};

use handlebars::Context;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use tokio::fs;

use agent_utils::KeyVault;
use windows_agent_client::{client::ClientBuilder, Command, WmiCommand};
use winrm_rs::{
    authentication::{Authentication, BasicAuth, KerberosAuth, NtlmAuth},
    responses::ps::PsOutput,
    session::{CertificateFormat, Session, SessionBuilder},
};

use crate::{
    error::{DTEResult, DTError},
    Error, Result,
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    connection: ConnectionConfig,
    script_context: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ConnectionConfig {
    WinRM(WinrmConfig),
    WindowsAgent(WindowsAgentConfig),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WindowsAgentConfig {
    pub hostname: String,
    #[serde(default = "default_wagent_port")]
    pub port: u16,
    pub server_root_cert: PathBuf,
    pub credentials: Option<Credentials>,
    #[serde(default = "default_timeout")]
    pub connection_timeout: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WinrmConfig {
    pub hostname: String,
    #[serde(default)]
    pub ip_address: Option<IpAddr>,
    pub credentials: Option<Credentials>,
    #[serde(default = "default_true")]
    pub https: bool,
    pub port: Option<u16>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    pub certificate: Option<(CertificateFormat, PathBuf)>,
    pub host_allias: Option<(HostAllias, Option<String>)>,
    #[serde(default)]
    pub disable_hostname_verification: bool,
    #[serde(default)]
    pub disable_certificate_verification: bool,
    #[serde(default = "default_true")]
    built_in_root_certs: bool,
    #[serde(default)]
    pub options: Option<WinRMOptions>,
}

pub enum WindowsSession {
    Winrm(winrm_rs::Session),
    WindowsAgent(windows_agent_client::client::Client),
}

#[derive(Debug, Default)]
pub struct CommandOutput {
    pub exitcode: i32,
    pub stderr: String,
    pub stdout: String,
}

impl CommandOutput {
    pub fn into_result(self) -> DTEResult<Self> {
        if self.exitcode == 0 {
            Ok(self)
        } else {
            Err(DTError::CommandFailed(self.exitcode, self.stderr))
        }
    }
}

impl From<PsOutput> for CommandOutput {
    fn from(value: PsOutput) -> Self {
        Self {
            exitcode: value.exitcode,
            stdout: value.stdout.join("\n"),
            stderr: value.stderr,
        }
    }
}

impl WindowsSession {
    pub async fn run_ps(&mut self, script: &str) -> DTEResult<CommandOutput> {
        match self {
            Self::Winrm(session) => {
                let shell = session.shell().await?;
                let result = session.run_ps(&shell, script).await;
                if let Err(e) = session.close_shell(shell).await {
                    warn!("Cannot close shell: {e}");
                }
                result.map(CommandOutput::from).map_err(DTError::Winrm)
            }
            Self::WindowsAgent(agent) => agent
                .request(Command::Powershell(script.to_string()))
                .await
                .map(|stdout| CommandOutput {
                    stdout,
                    ..Default::default()
                })
                .map_err(DTError::WindowsAgent),
        }
    }

    pub async fn get_wmiobject(
        &mut self,
        class: &str,
        namespace: &str,
        attributes: &[String],
    ) -> DTEResult<Vec<HashMap<String, String>>> {
        match self {
            WindowsSession::Winrm(session) => {
                let shell = session.shell().await?;
                let result = session
                    .get_wmiobject(&shell, class, attributes, namespace)
                    .await;
                if let Err(e) = session.close_shell(shell).await {
                    warn!("Cannot close shell: {e}");
                }
                result.map_err(DTError::Winrm)
            }
            WindowsSession::WindowsAgent(agent) => {
                let command = Command::WMI(WmiCommand {
                    namespace: namespace.to_string(),
                    class: class.to_string(),
                    attributes: attributes.to_vec(),
                });
                let output = agent
                    .request(command)
                    .await
                    .map_err(DTError::WindowsAgent)?;

                let output: Vec<HashMap<String, serde_json::Value>> =
                    serde_json::from_str(&output)
                        .map_err(DTError::WindowsAgentOutput)?;
                let output: Vec<HashMap<String, String>> = output
                    .into_iter()
                    .map(|row| {
                        row.into_iter()
                            .map(|(k, v)| {
                                (
                                    k,
                                    match v {
                                        serde_json::Value::Null => {
                                            String::new()
                                        }
                                        serde_json::Value::String(s) => s,
                                        v => v.to_string(),
                                    },
                                )
                            })
                            .collect()
                    })
                    .collect();
                Ok(output)
            }
        }
    }

    pub async fn get_ciminstance(
        &mut self,
        class: &str,
        namespace: &str,
        attributes: &[String],
    ) -> DTEResult<Vec<HashMap<String, String>>> {
        match self {
            WindowsSession::Winrm(session) => {
                let shell = session.shell().await?;
                let result = session
                    .get_ciminstance(&shell, class, attributes, namespace)
                    .await;
                if let Err(e) = session.close_shell(shell).await {
                    warn!("Cannot close shell: {e}");
                };
                result.map_err(DTError::Winrm)
            }
            WindowsSession::WindowsAgent(_) => {
                self.get_wmiobject(class, namespace, attributes).await
            }
        }
    }

    pub async fn enumerate_ciminstance(
        &mut self,
        class: &str,
        namespace: &str,
        attributes: &[String],
    ) -> DTEResult<Vec<HashMap<String, String>>> {
        match self {
            WindowsSession::Winrm(session) => session
                .enumerate_ciminstance(class, namespace)
                .await
                .map_err(DTError::Winrm),
            WindowsSession::WindowsAgent(_) => {
                self.get_wmiobject(class, namespace, attributes).await
            }
        }
    }
}

impl Config {
    pub async fn new_session(
        &self,
        keyvault: &KeyVault,
    ) -> Result<WindowsSession> {
        self.connection.new_session(keyvault).await
    }
    pub fn script_context(&self) -> Context {
        Context::wraps(&self.script_context).unwrap()
    }
}

impl ConnectionConfig {
    pub async fn new_session(
        &self,
        keyvault: &KeyVault,
    ) -> Result<WindowsSession> {
        match self {
            Self::WinRM(cnf) => {
                cnf.get_session(keyvault).await.map(WindowsSession::Winrm)
            }

            Self::WindowsAgent(cnf) => cnf
                .get_session(keyvault)
                .await
                .map(WindowsSession::WindowsAgent),
        }
    }
}

impl WindowsAgentConfig {
    pub async fn get_session(
        &self,
        _keyvault: &KeyVault,
    ) -> Result<windows_agent_client::client::Client> {
        let creds = self.credentials.as_ref().ok_or(Error::NoCredentials)?;

        let (private, public) = if let Credentials::Certificate(cauth) = creds {
            (&cauth.private_key, &cauth.public_cert)
        } else {
            return Err(Error::UnsupportedCredentials);
        };

        debug!(
            "loading public_cert: {}, private_key: {}",
            public.display(),
            private.display()
        );
        debug!("loading ca: {}", self.server_root_cert.display());

        ClientBuilder::new(self.hostname.clone())
            .set_timeout(self.connection_timeout)
            .load_client_cert(public, private)
            .await?
            .load_root_ca(&self.server_root_cert)
            .await?
            .set_port(self.port)
            .build()
            .await
            .map_err(Error::WindowsAgent)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WinRMOptions {
    /// If set to TRUE, this option specifies that the user profile does not exist on the remote system and that the default profile SHOULD be used.
    /// By default, the value is TRUE.
    #[serde(default = "default_true")]
    noprofile: bool,
    /// If set to TRUE, this option requests that the server runs the command without using cmd.exe;
    /// if set to FALSE, the server is requested to use cmd.exe.
    /// By default the value is FALSE.
    /// This does not have any impact on the wire protocol.
    #[serde(default)]
    skip_cmd_shell: bool,
}

fn default_true() -> bool {
    true
}
fn default_wagent_port() -> u16 {
    8099
}
fn default_timeout() -> u64 {
    10
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum HostAllias {
    Domain,
    Ip,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Credentials {
    Basic(BasicCredentials),
    Ntlm(NtlmCredentials),
    Kerberos(KerberosCredentials),
    Certificate(CertificateCredentials),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct BasicCredentials {
    pub username: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NtlmCredentials {
    pub username: String,
    pub password: String,
    pub domain: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KerberosCredentials {
    pub hostname: String,
    pub realm: String,
    pub ccache_name: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CertificateCredentials {
    pub private_key: PathBuf,
    pub public_cert: PathBuf,
}

impl Credentials {
    pub async fn to_authentication(
        &self,
        key_vault: &KeyVault,
    ) -> Result<Authentication> {
        info!("Using authentication: {}", self);
        match self {
            Credentials::Kerberos(kauth) => {
                Ok(Authentication::Kerberos(KerberosAuth::new(
                    &kauth.hostname,
                    kauth.realm.to_uppercase(),
                    kauth.ccache_name.as_ref(),
                )))
            }
            Credentials::Basic(bauth) => {
                let (username, password) = Self::from_keyvault(
                    key_vault,
                    bauth.username.to_string(),
                    &bauth.password,
                )
                .await?;
                Ok(Authentication::Basic(BasicAuth::new(username, password)))
            }
            Credentials::Ntlm(nauth) => {
                let (username, password) = Self::from_keyvault(
                    key_vault,
                    nauth.username.to_string(),
                    &nauth.password,
                )
                .await?;
                Ok(Authentication::Ntlm(if let Some(domain) = &nauth.domain {
                    NtlmAuth::new(username, domain.clone(), password)
                } else {
                    NtlmAuth::domainless(username, password)
                }))
            }
            Credentials::Certificate { .. } => {
                Err(Error::UnsupportedCredentials)
            }
        }
    }

    pub async fn from_keyvault(
        key_vault: &KeyVault,
        entry: String,
        default: &String,
    ) -> Result<(String, String)> {
        match key_vault {
            KeyVault::Identity => Ok((entry, default.to_string())),
            _ => {
                let kr_entry = key_vault
                    .retrieve_creds(entry)
                    .await?
                    .ok_or(Error::MissingKREntry)?;
                let username = kr_entry
                    .username
                    .as_ref()
                    .ok_or(Error::MissingKRObject(String::from("username")))?
                    .split('@')
                    .next()
                    .ok_or(Error::MissingKRObject(String::from("username")))?
                    .to_string();
                let password = kr_entry
                    .password
                    .as_ref()
                    .ok_or(Error::MissingKRObject(String::from("password")))?
                    .to_string();
                Ok((username, password))
            }
        }
    }
}

impl std::fmt::Display for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (auth, user) = match self {
            Credentials::Basic(bauth) => ("Basic", bauth.username.as_str()),
            Credentials::Ntlm(nauth) => ("Ntml", nauth.username.as_str()),
            Credentials::Kerberos(kauth) => ("Kerberos", kauth.realm.as_str()),
            Credentials::Certificate(cauth) => (
                "Certificate",
                cauth
                    .private_key
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default(),
            ),
        };
        write!(
            f,
            "{} with {} {}",
            match self {
                Credentials::Kerberos { .. } => "realm",
                _ => "user",
            },
            auth,
            user
        )
    }
}

impl WinrmConfig {
    async fn get_credentials(
        &self,
        key_vault: &KeyVault,
    ) -> Result<Authentication> {
        self.credentials
            .as_ref()
            .ok_or(Error::NoCredentials)?
            .to_authentication(key_vault)
            .await
    }

    async fn get_hostname(&self) -> Result<String> {
        match &self.host_allias {
            Some((HostAllias::Domain, Some(domain))) => {
                Ok(format!("{}.{}", self.hostname, domain))
            }
            Some((HostAllias::Ip, Some(ip))) => Ok(ip.clone()),
            Some((HostAllias::Ip, None)) => {
                Ok(agent_utils::ip_lookup_one(&self.hostname)
                    .await?
                    .to_string())
            }
            _ => Ok(self.hostname.clone()),
        }
    }

    pub fn split_ssl_file_per_ssl_cert(ssl_file: Vec<u8>) -> Vec<Vec<u8>> {
        // Split a read-in ssl file into multiple certificates.
        let hay = String::from_utf8_lossy(&ssl_file);

        let certificates: Vec<Vec<u8>> = hay
            .split("-----BEGIN CERTIFICATE-----")
            .map(|s| {
                format!("{}{}", "-----BEGIN CERTIFICATE-----", s)
                    .as_bytes()
                    .to_vec()
            })
            .collect();

        // the split function will return an empty string from before the first -----BEGIN CERTIFICATE-----
        // We are basically removing this empty string here
        let mut output: Vec<Vec<u8>> = vec![];
        for ssl in certificates {
            if ssl.len() > 50 {
                output.push(ssl);
            }
        }
        output
    }

    pub async fn get_session(&self, key_vault: &KeyVault) -> Result<Session> {
        info!("creating winrm session for: {}", self.get_hostname().await?);
        let credentials = self.get_credentials(key_vault).await?;

        let mut session_builder = SessionBuilder::with_credentials(credentials)
            .hostname(self.get_hostname().await?.to_lowercase())
            .https(self.https)
            .timeout(self.timeout)
            .ignore_hostnames(self.disable_hostname_verification)
            .ignore_cert(self.disable_certificate_verification)
            .built_in_root_certs(self.built_in_root_certs);
        if let Some(addr) = self.ip_address {
            session_builder = session_builder.resolve(addr);
        }
        if let Some(port) = self.port {
            session_builder = session_builder.port(port);
        }
        if let Some((cert_type, cert_path)) = &self.certificate {
            let ssl_file = fs::read(cert_path).await?;
            for ssl in matches!(cert_type, CertificateFormat::PEM)
                .then(|| {
                    WinrmConfig::split_ssl_file_per_ssl_cert(ssl_file.clone())
                })
                .unwrap_or(vec![ssl_file])
            {
                session_builder = session_builder
                    .root_ca(cert_type.clone(), &ssl)
                    .map_err(|_| {
                        Error::Custom(format!(
                            "{} is not a valid {:?} certificate",
                            cert_path.display(),
                            cert_type.clone()
                        ))
                    })?;
            }
        }

        if let Some(opts) = &self.options {
            session_builder = session_builder
                .noprofile(opts.noprofile)
                .skip_cmd_shell(opts.skip_cmd_shell);
        }

        let mut session = session_builder.build()?;
        session.login().await?;
        Ok(session)
    }
}
