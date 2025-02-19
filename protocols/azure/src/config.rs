/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::error::{AzureError, Result};

use std::collections::HashMap;

use agent_utils::KeyVault;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use uritemplate::UriTemplate;

use rest_protocol::{config::Application, http::*, Template};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
#[derive(Default)]
pub struct Config {
    #[serde(rename = "client")]
    pub client: Option<ClientInfo>,
    #[serde(rename = "subscriptions")]
    pub subscriptions: Option<Vec<String>>,
    #[serde(rename = "resourceGroups")]
    pub resource_groups: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientInfo {
    #[serde(rename = "tenantId")]
    pub tenant_id: Option<String>,
    #[serde(rename = "ClientName")]
    pub client_name: Option<String>,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "clientSecret")]
    pub client_secret: Option<String>,
}

impl Config {
    pub async fn login(&self, vault: Option<&KeyVault>) -> Result<Client> {
        if let Some(cl) = &self.client {
            cl.login(vault).await
        } else {
            Err(AzureError::NoPassword)
        }
    }
}

impl ClientInfo {
    pub async fn login(&self, vault: Option<&KeyVault>) -> Result<Client> {
        let mut data_template: HashMap<String, Template> = HashMap::new();
        data_template.insert(
            String::from("grant_type"),
            Template::parse("client_credentials")?,
        );
        data_template.insert(
            String::from("client_secret"),
            Template::parse("{{clientSecret}}")?,
        );
        data_template.insert(
            String::from("client_id"),
            Template::parse("{{clientId}}")?,
        );
        data_template.insert(
            String::from("resource"),
            Template::parse("https://management.azure.com/")?,
        );

        let mut credentials: HashMap<String, String> = HashMap::new();
        credentials.insert(
            String::from("tenantId"),
            self.tenant_id.clone().unwrap_or_default(),
        );
        credentials.insert(String::from("clientId"), self.client_id.clone());
        let secret = match vault.unwrap_or(&KeyVault::Identity) {
            KeyVault::Identity => self
                .client_secret
                .as_ref()
                .ok_or(AzureError::NoPassword)?
                .clone(),
            KeyVault::KeyReader(vault) => {
                vault
                    .retrieve_password(
                        self.client_name
                            .as_ref()
                            .ok_or(AzureError::NoPassword)?
                            .clone(),
                    )
                    .await?
            }
        };
        credentials.insert(String::from("clientSecret"), secret);

        let mut rest_application: Application = Application {
            content_type: ContentType::JSON,
            auth_type: AuthType::Token(Template::parse(
                "Bearer {{access_token}}",
            )?),
            login_url: UriTemplate::new(
                "https://login.microsoftonline.com/{tenantId}/oauth2/token",
            ),
            login_method: HTTPMethod::POST,
            login_body_type: BodyType::FormUrlEncoded,
            login_data: data_template,
        };

        rest_application
            .login(&credentials)
            .await
            .map_err(AzureError::RESTError)
    }
}
