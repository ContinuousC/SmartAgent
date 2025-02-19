/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use agent_utils::KeyVault;
use log::{debug, error, trace};
use protocol::{
    auth::{LookupKeyvault, NtlmAuth},
    http,
};
use serde::de::DeserializeOwned;
use tap::TapFallible;

use super::{Config, DTEResult, Result};

pub struct Client {
    inner: reqwest::Client,
    auth: NtlmAuth,
    base_url: String,
    hostentry: Option<String>,
}

impl Client {
    pub async fn new(config: &Config, keyvault: KeyVault) -> Result<Self> {
        // Monitor/OData/v2/Data/Machines
        let base_url = format!(
            "{}://{}:{}/Citrix",
            config.http.scheme(),
            http::Config::alias_host(
                config.http.host_allias.as_ref(),
                config
                    .director_server
                    .as_deref()
                    .unwrap_or(config.http.hostname.as_str()),
                config.http.ipaddress.as_ref()
            )
            .await?,
            config.http.http_port()
        );
        let auth = config.auth.lookup_keyvault(keyvault).await?;
        let (client, _) = config.http.create_client(Vec::new()).await?;
        let hostentry = config.director_server.as_ref().map(|_| {
            format!(
                "{}\\{}",
                auth.domain
                    .as_deref()
                    .unwrap_or(config.http.hostname.as_str()),
                &config.http.hostname
            )
            .to_uppercase()
        });

        Ok(Self {
            inner: client,
            auth,
            base_url,
            hostentry,
        })
    }

    pub async fn request<T: DeserializeOwned>(
        &self,
        endpoint: &str,
        expand: &str,
    ) -> DTEResult<T> {
        let mut query_builder = vec![format!("$expand={expand}")];
        if let Some(entry) = &self.hostentry {
            query_builder.push(format!("$filter=Name%20eq%20'{entry}'"));
        }
        let url = format!(
            "{}/{}?{}",
            self.base_url,
            endpoint,
            query_builder.join("&")
        );
        debug!("requesting url: {url}");

        let body = self
            .auth
            .get_request(&self.inner, &url)
            .await
            .tap_err(|e| error!("error while requesting data: {e:?}"))
            .tap_ok(|body| trace!("data recieved from {url}:\n{body}"))?;

        let data: T = quick_xml::de::from_str(&body)
            .tap_err(|e| error!("error while deserializing data: {e:?}"))?;

        Ok(data)
    }
}
