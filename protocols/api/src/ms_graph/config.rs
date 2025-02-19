/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use reqwest::Client;

use agent_utils::KeyVault;
use log::warn;
use rest_protocol::{
    http::{AuthType, BodyType, ContentType, HTTPMethod},
    Application, Template,
};
use serde::{Deserialize, Serialize};
use uritemplate::UriTemplate;

use crate::ms_graph::filters::{OnedriveUsage, OutlookUsage, SharepointUsage};
use crate::ms_graph::{Error, Result};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Config {
    pub credentials: Option<Credentials>,
    #[serde(default)]
    pub rapports: Option<RapportConfig>,
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Credentials {
    pub tenant_id: Option<String>,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_name: Option<String>,
}

impl Config {
    pub async fn login(&self, vault: &KeyVault) -> Result<Client> {
        if let Some(cl) = &self.credentials {
            cl.login(Some(vault)).await
        } else {
            Err(Error::NoPassword)
        }
    }
}

impl Credentials {
    pub async fn login(&self, vault: Option<&KeyVault>) -> Result<Client> {
        let mut data_template: HashMap<String, Template> = HashMap::new();
        data_template.insert(
            String::from("grant_type"),
            Template::parse("client_credentials")?,
        );
        data_template.insert(
            String::from("client_secret"),
            Template::parse("{{client_secret}}")?,
        );
        data_template.insert(
            String::from("client_id"),
            Template::parse("{{client_id}}")?,
        );
        data_template.insert(
            String::from("scope"),
            Template::parse("https://graph.microsoft.com/.default")?,
        );

        let mut credentials: HashMap<String, String> = HashMap::new();
        if let Some(tenant_id) = self.tenant_id.as_ref() {
            credentials.insert(String::from("tenant_id"), tenant_id.clone());
        } else {
            warn!("No tenant given!");
        }
        credentials.insert(String::from("client_id"), self.client_id.clone());
        credentials.insert(
            String::from("client_secret"),
            match vault.unwrap_or(&KeyVault::Identity) {
                KeyVault::Identity => {
                    self.client_secret.as_ref().cloned().unwrap_or_default()
                }
                KeyVault::KeyReader(vault) => {
                    vault
                        .retrieve_password(
                            self.client_name
                                .as_ref()
                                .cloned()
                                .unwrap_or_default(),
                        )
                        .await?
                }
            },
        );

        let mut rest_application = Application {
            content_type: ContentType::JSON,
            auth_type: AuthType::Token(Template::parse(
                "Bearer {{access_token}}"
            )?),
            login_url: UriTemplate::new("https://login.microsoftonline.com/{tenant_id}/oauth2/v2.0/token"),
            login_method: HTTPMethod::POST,
            login_body_type: BodyType::FormUrlEncoded,
            login_data: data_template
        };

        rest_application
            .login(&credentials)
            .await
            .map_err(Error::RESTError)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RapportConfig {
    #[serde(default = "default_onedrive_usage")]
    pub onedrive_usage: (usize, OnedriveUsage),
    #[serde(default = "default_outlook_usage")]
    pub outlook_usage: (usize, OutlookUsage),
    #[serde(default = "default_sharepoint_usage")]
    pub sharepoint_usage: (usize, SharepointUsage),
}

impl Default for RapportConfig {
    fn default() -> Self {
        Self {
            onedrive_usage: default_onedrive_usage(),
            outlook_usage: default_outlook_usage(),
            sharepoint_usage: default_sharepoint_usage(),
        }
    }
}
fn default_onedrive_usage() -> (usize, OnedriveUsage) {
    (200, OnedriveUsage::default())
}
fn default_outlook_usage() -> (usize, OutlookUsage) {
    (200, OutlookUsage::default())
}
fn default_sharepoint_usage() -> (usize, SharepointUsage) {
    (200, SharepointUsage::default())
}
