/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt::Display;
use std::mem;

use log::{debug, error, info};
use protocol::auth::{BasicAuth, LookupKeyvault};
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::Response;
use serde::de::DeserializeOwned;

use agent_utils::KeyVault;
use protocol::http::Config;
use tap::TapFallible;

use crate::mirth::{ApiError as Error, ApiResult as Result};

pub struct Client {
    base_url: String,
    client: reqwest::Client,
    auth: BasicAuth,
}

impl Client {
    pub async fn new(
        config: &Config,
        creds: &BasicAuth,
        key_vault: KeyVault,
    ) -> Result<Self> {
        let base_url =
            format!("{}/{}", config.base_url(Some(55502)).await?, "api");

        let auth = creds.lookup_keyvault(key_vault).await?;
        if auth.is_empty() {
            return Err(Error::NoCredentials);
        }

        let (client, _) = config
            .create_client(vec![(
                HeaderName::from_static("x-requested-with"),
                HeaderValue::from_static("MirthAgent"),
            )])
            .await?;

        Ok(Client {
            base_url,
            client,
            auth,
        })
    }

    async fn handle_response(&self, response: Response) -> Result<String> {
        let status = response.error_for_status_ref().map(|_| ());
        let body = response.text().await.map_err(Error::RetrieveBody)?;

        status
            .map(|_| body.clone())
            .map_err(|e| Error::InvalidResponse(body, e))
    }

    pub async fn login(&self) -> Result<()> {
        let endpoint = format!("{}/{}", self.base_url, "users/_login");
        info!("logging in with username: {}", &self.auth.username);
        debug!("using url: {endpoint}");

        let response = self
            .client
            .post(endpoint)
            .basic_auth(&self.auth.username, self.auth.password.as_deref())
            .form(&[
                ("username", self.auth.username.as_str()),
                ("password", self.auth.password.as_deref().unwrap()),
            ])
            .send()
            .await
            .map_err(Error::SendRequest)
            .tap_err(|e| error!("cannot send requests to api: {e:?}"))?;

        self.handle_response(response)
            .await
            .map(mem::drop)
            .tap_ok(|_| info!("loging successfull"))
            .tap_err(|e| error!("login failed: {e:?}"))
    }

    pub async fn get_endpoint<T: DeserializeOwned>(
        &self,
        endpoint: impl Display,
    ) -> Result<T> {
        let url = format!("{}/{}", self.base_url, endpoint);
        debug!("retrieving data from endpoint: {url}");
        let response = self
            .client
            .get(&url)
            .basic_auth(&self.auth.username, self.auth.password.as_deref())
            .send()
            .await
            .map_err(Error::SendRequest)?;

        let body = self.handle_response(response).await?;
        serde_xml_rs::from_str(&body).map_err(Error::DeserializeBody)
    }
}
