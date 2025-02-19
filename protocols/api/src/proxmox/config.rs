/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::sync::Arc;

use log::{error, info};
use protocol::{
    auth::{self, LookupKeyvault},
    http,
};
use serde::{Deserialize, Serialize};
use tap::TapFallible;

use super::{Client, Plugin, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    auth: auth::BasicAuth,
    http: http::Config,
}

impl Config {
    pub async fn get_client<'a>(
        &self,
        plugin: &'a Plugin,
    ) -> Result<Arc<Client<'a>>> {
        let base_url =
            format!("{}/api2/json", self.http.base_url(Some(8006)).await?);

        let auth = self.auth.lookup_keyvault(plugin.key_vault.clone()).await?;

        // use reqwest::header::{self, HeaderValue};
        // let auth = format!(
        //     "PVEAPIToken={}={}",
        //     &auth.username,
        //     auth.password.as_ref().unwrap()
        // );
        // let auth = vec![(
        //     header::AUTHORIZATION,
        //     HeaderValue::from_str(&auth).unwrap(),
        // )];

        let (client, cookiejar) = self
            .http
            // .create_client(auth)
            .create_client(Vec::new())
            .await?;

        let client = Client::new(client, &self.http.hostname, base_url, plugin);
        client
            .login(auth, cookiejar)
            .await
            .tap_ok(|_| info!("login successfull"))
            .tap_err(|e| error!("failed to log in: {e}"))?;

        Ok(Arc::new(client))
    }
}
