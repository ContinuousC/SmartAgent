/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    net::IpAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use agent_utils::{vault::Creds, KeyVault};
use async_ssh2_lite::{AsyncSession, SessionConfiguration, TokioTcpStream};
use log::trace;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tap::Pipe;

use crate::{Error, Result};
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub connectivity: Connectivity,
    pub credentials: Credential,
    #[serde(default)]
    pub options: Options,
    #[serde(default)]
    pub jumphosts: Vec<JumpHost>, // ToDo - not yet implemented
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Connectivity {
    pub hostname: String,
    #[serde(default)]
    pub ipaddress: Option<IpAddr>,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_max_sessions")]
    pub max_sessions: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Options {
    #[serde(default = "default_sudo")]
    pub allow_sudo: bool,
    #[serde(default = "default_timeout")]
    pub timeout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JumpHost {
    connectivity: Connectivity,
    credentials: Credential,
}

fn default_port() -> u16 {
    22
}

fn default_timeout() -> u32 {
    10
}

fn default_sudo() -> bool {
    false
}

fn default_max_sessions() -> u8 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    IdentityFile {
        identity_file: PathBuf,
        password: Option<String>,
    },
    Password {
        password: String,
    },
}
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Credential {
    pub username: String,
    pub credential_type: Option<CredentialType>,
}

impl Config {
    pub async fn get_session(
        &self,
        key_vault: &KeyVault,
    ) -> Result<Arc<AsyncSession<TokioTcpStream>>> {
        if self.credentials.username.is_empty() {
            return Err(Error::NoCredentialsProvided);
        }

        let ip = if let Some(ip) = self.connectivity.ipaddress {
            ip
        } else {
            agent_utils::ip_lookup_one(&self.connectivity.hostname).await?
        };

        log::trace!(
            "Connecting to host {} on port {} on ip {}",
            self.connectivity.hostname.to_string(),
            self.connectivity.port.to_string(),
            ip.to_string()
        );

        let mut config = SessionConfiguration::new();
        config.set_timeout(self.options.timeout * 1000);

        let mut session =
            AsyncSession::<async_ssh2_lite::TokioTcpStream>::connect(
                SocketAddr::from((ip, self.connectivity.port)),
                config,
            )
            .await
            .map_err(|e| Error::Connection(e, self.connectivity.port))?;
        log::info!("SSH session created");

        session
            .handshake()
            .await
            .map_err(Error::AuthenticationFailed)?;
        log::info!("SSH session handshaked");

        // Authenticate with keyvault / password / identity file
        match key_vault {
            KeyVault::KeyReader(_) => {
                log::info!("Start session auth with keyvault");
                let creds = key_vault
                    .retrieve_creds(self.credentials.username.clone())
                    .await
                    .map_err(Error::KeyReader)?
                    .ok_or(Error::KeyReaderCredentials)?;
                self.session_auth_with_keyvault(creds, &session).await?;
            }

            // Use wato config
            KeyVault::Identity => match &self.credentials.credential_type {
                Some(credential_type) => {
                    match credential_type {
                        CredentialType::IdentityFile {
                            identity_file,
                            password,
                        } => {
                            log::info!("Start session auth with identityfile");
                            self.session_auth_with_identityfile(
                                self.credentials.username.clone(),
                                identity_file,
                                password,
                                &session,
                            )
                            .await?
                        }
                        CredentialType::Password { password } => {
                            log::info!("Start session auth with password");
                            self.session_auth_with_password(
                                password,
                                self.credentials.username.clone(),
                                &session,
                            )
                            .await?
                        }
                    };
                }
                None => {
                    return Err(Error::NoCredentialsProvided);
                }
            },
        };

        if !session.authenticated() {
            return Err(Error::NotAuthenticated);
        }

        log::info!("SSH session succesfully authenticated");
        Ok(Arc::new(session))
    }

    pub async fn session_auth_with_keyvault(
        &self,
        creds: Creds,
        session: &AsyncSession<TokioTcpStream>,
    ) -> Result<()> {
        trace!("authenticating using keyvault");
        let username = &creds.username.ok_or(Error::NoCredentialsProvided)?;
        match (creds.key, creds.password) {
            (Some(key), _) => {
                trace!("authenticating as user {username} with key");
                session
                    .userauth_pubkey_memory(username, None, key.as_ref(), None)
                    .await
                    .map_err(Error::AuthenticationFailed)?
                    .pipe(Ok)
            }
            (None, Some(password)) => {
                trace!("authenticating as user {username} with password");
                self.session_auth_with_password(
                    &password,
                    username.to_string(),
                    session,
                )
                .await?
                .pipe(Ok)
            }
            (None, None) => Err(Error::KeyReaderCredentials),
        }
    }

    pub async fn session_auth_with_identityfile(
        &self,
        username: String,
        file_location: &Path,
        password: &Option<String>,
        session: &AsyncSession<TokioTcpStream>,
    ) -> Result<()> {
        trace!("authenticating as user {username} with key: {file_location:?}");
        session
            .userauth_pubkey_file(
                &username,
                None,
                file_location,
                password.as_ref().map(|x| x.as_str()),
            )
            .await
            .map_err(Error::AuthenticationFailed)?;
        Ok(())
    }

    pub async fn session_auth_with_password(
        &self,
        password: &str,
        username: String,
        session: &AsyncSession<TokioTcpStream>,
    ) -> Result<()> {
        trace!("authenticating as user {username} with password");
        session
            .userauth_password(&username, password)
            .await
            .map_err(Error::AuthenticationFailed)?;
        Ok(())
    }
}
