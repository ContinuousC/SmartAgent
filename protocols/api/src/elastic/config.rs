/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use agent_utils::KeyVault;
use protocol::{
    auth::{self, LookupKeyvault},
    http,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub http: http::Config,
    pub auth: auth::BasicAuth,
}

impl Config {
    pub async fn get_client(&self) -> Result<Client> {
        Ok(self.http.create_client(Vec::new()).await?.0)
    }
    pub async fn get_credentials(
        &self,
        keyvault: KeyVault,
    ) -> Result<auth::BasicAuth> {
        self.auth
            .lookup_keyvault(keyvault)
            .await
            .map_err(Error::CredentialLookup)
    }
}
