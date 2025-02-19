/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use agent_utils::KeyVault;
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Entry not found in keyvault")]
    MissingKREntry,
    #[error("No {0} in KeyVault entry")]
    MissingKRObject(String),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("An empty password was provided while not using a keyvault")]
    EmptyPassword,
}

async fn get_keyvault_entry(
    keyvault: KeyVault,
    path: String,
    default: impl ToString,
) -> Result<BasicAuth> {
    match keyvault {
        KeyVault::Identity => {
            let password = default.to_string();
            if password.is_empty() {
                Err(Error::EmptyPassword)
            } else {
                Ok(BasicAuth::new(path, password))
            }
        }

        _ => {
            let kr_entry = keyvault
                .retrieve_creds(path)
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

            Ok(BasicAuth::new(username, password))
        }
    }
}

#[async_trait::async_trait]
pub trait LookupKeyvault {
    async fn lookup_keyvault(&self, keyvault: KeyVault) -> Result<Self>
    where
        Self: Sized;
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct BasicAuth {
    pub username: String,
    pub password: Option<String>,
}

impl BasicAuth {
    pub fn new(username: String, password: String) -> Self {
        Self {
            username,
            password: Some(password),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.username.is_empty()
            || matches!(self.password.as_deref(), None | Some(""))
    }
}

#[async_trait::async_trait]
impl LookupKeyvault for BasicAuth {
    async fn lookup_keyvault(&self, keyvault: KeyVault) -> Result<Self>
    where
        Self: Sized,
    {
        let password = self.password.as_deref().unwrap_or_default();
        get_keyvault_entry(keyvault, self.username.clone(), password).await
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NtlmAuth {
    pub username: String,
    pub password: Option<String>,
    pub domain: Option<String>,
}

impl NtlmAuth {
    pub fn new(username: String, password: String, domain: String) -> Self {
        Self {
            username,
            password: Some(password),
            domain: Some(domain),
        }
    }

    pub fn no_domain(username: String, password: String) -> Self {
        Self {
            username,
            password: Some(password),
            domain: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.username.is_empty()
            || matches!(self.password.as_deref(), None | Some(""))
    }
}

#[async_trait::async_trait]
impl LookupKeyvault for NtlmAuth {
    async fn lookup_keyvault(&self, keyvault: KeyVault) -> Result<Self>
    where
        Self: Sized,
    {
        let password = self.password.as_deref().unwrap_or_default();
        let basic =
            get_keyvault_entry(keyvault, self.username.clone(), password)
                .await?;

        Ok(Self {
            username: basic.username,
            password: basic.password,
            domain: self.domain.clone(),
        })
    }
}

#[cfg(feature = "reqwest")]
pub mod reqwest {
    use base64::{engine::general_purpose, Engine};
    use log::{info, warn};
    use rdp::nla::{ntlm::Ntlm, sspi::AuthenticationProtocol};
    use reqwest::{Client, StatusCode};

    use super::NtlmAuth;

    pub type NtlmResult<T> = std::result::Result<T, NtlmError>;

    #[derive(Debug, thiserror::Error)]
    pub enum NtlmError {
        #[error("Error during HTTP-request: {0}")]
        ReqwestError(#[from] reqwest::Error),
        #[error("No authenticate header found in stage: {0}")]
        NoAuthenticateHeader(String),
        #[error("Invalid authenticate header found in stage: {0}")]
        InvalidAuthenticateHeader(String),
        #[error("error while parsing messages {0:?}")]
        RdpError(rdp::model::error::Error),
        #[error("error while decoding base64 {0}")]
        DecodeError(#[from] base64::DecodeError),
        #[error("Unable to log in with given credentials")]
        InvalidCredentials,
        #[error("{0}")]
        Custom(String),
    }

    impl From<rdp::model::error::Error> for NtlmError {
        fn from(e: rdp::model::error::Error) -> Self {
            NtlmError::RdpError(e)
        }
    }

    enum Authentication {
        Std,
        Proxy,
    }

    impl Authentication {
        fn server_header(&self) -> String {
            match self {
                Authentication::Std => String::from("www-authenticate"),
                Authentication::Proxy => String::from("proxy-authenticate"),
            }
        }

        fn client_header(&self) -> String {
            match self {
                Authentication::Std => String::from("Authorization"),
                Authentication::Proxy => String::from("Proxy-authorization"),
            }
        }
    }

    impl NtlmAuth {
        // https://docs.microsoft.com/en-us/openspecs/windows_protocols/ms-ntht/f09cf6e1-529e-403b-a8a5-7368ee096a6a
        pub async fn get_request(
            &self,
            client: &Client,
            url: &str,
        ) -> NtlmResult<String> {
            let initial_response =
                client.get(url).header("Content-Length", "0").send().await?;
            let auth = match initial_response.status() {
                StatusCode::UNAUTHORIZED => Authentication::Std,
                StatusCode::PROXY_AUTHENTICATION_REQUIRED => {
                    Authentication::Proxy
                }
                _ => {
                    return Err(NtlmError::Custom(format!(
                    "Recieved an unexpected statud code from the server: {}",
                    initial_response.status()
                )))
                }
            };
            let auth_type = initial_response
                .headers()
                .get(auth.server_header())
                .ok_or_else(|| {
                    NtlmError::NoAuthenticateHeader(String::from("nagotiation"))
                })?
                .to_str()
                .unwrap();

            let mut ntlm = Ntlm::new(
                self.domain.as_deref().unwrap_or_default().to_string(),
                self.username.to_string(),
                self.password.as_deref().unwrap_or_default().to_string(),
            );
            let negotiate_message = ntlm
                .create_negotiate_message()
                .map_err(NtlmError::RdpError)?;
            let negotiate_response = client
                .post(url)
                .header("Content-Length", "0")
                .header(
                    &auth.client_header(),
                    format!(
                        "{} {}",
                        &auth_type,
                        general_purpose::STANDARD.encode(negotiate_message)
                    ),
                )
                .send()
                .await?;

            let challenge = negotiate_response
                .headers()
                .get(auth.server_header())
                .ok_or_else(|| {
                    NtlmError::NoAuthenticateHeader(String::from("Challenge"))
                })?
                .to_str()
                .unwrap()
                .split(' ')
                .nth(1)
                .ok_or_else(|| {
                    NtlmError::InvalidAuthenticateHeader(String::from(
                        "Challenge",
                    ))
                })?;
            let challenge_decoded =
                general_purpose::STANDARD.decode(challenge.as_bytes())?;

            let authenticate_message = ntlm
                .read_challenge_message(&challenge_decoded)
                .map_err(NtlmError::RdpError)?;
            let authenticate_response = client
                .get(url)
                .header("Content-Length", "0")
                .header(
                    &auth.client_header(),
                    format!(
                        "{} {}",
                        &auth_type,
                        general_purpose::STANDARD.encode(authenticate_message)
                    ),
                )
                .send()
                .await?;

            let status = authenticate_response.status();
            match status {
                StatusCode::UNAUTHORIZED
                | StatusCode::PROXY_AUTHENTICATION_REQUIRED => {
                    warn!("NTLM Authentication failed");
                    Err(NtlmError::InvalidCredentials)
                }
                StatusCode::OK => {
                    info!("NTLM Authentication succesfull");
                    Ok(authenticate_response.text().await?)
                }
                _ => {
                    let body = authenticate_response.text().await;
                    let err = format!(
                        "request failed! recieved statuscode {} ({}): {}",
                        status,
                        status.canonical_reason().unwrap_or("unkown"),
                        match body {
                            Ok(body) => body,
                            Err(e) => e.to_string(),
                        }
                    );
                    warn!("{err}");
                    Err(NtlmError::Custom(err))
                }
            }
        }
    }
}
