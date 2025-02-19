/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use log::debug;
use reqwest::Client;
use uritemplate::UriTemplate;

use crate::AzureError;
use rest_protocol::{http::*, input::RESTRequest, Template};
use serde::de::DeserializeOwned;

use crate::{ResourceResponse, Result, SubscriptionId};

pub async fn paged_requests<T: DeserializeOwned>(
    client: &Client,
    url: UriTemplate,
    tmpvars: HashMap<String, Template>,
    data: HashMap<String, String>,
) -> Result<Vec<T>> {
    debug!("recieved request");
    let mut request = RESTRequest {
        url,
        data: tmpvars,
        method: HTTPMethod::GET,
        schema: serde_json::Value::Null,
        reference: None,
    };
    let response = request.execute(client, &data).await?;
    debug!("response from azure: {}", &response);
    let mut response = serde_json::from_str::<ResourceResponse<T>>(&response)?;

    match response {
        ResourceResponse::Error(e) => Err(AzureError::Response(e)),
        ResourceResponse::Success(resources) => {
            let mut results = resources.value;
            let mut next_link = resources.next_link;
            while let Some(ref next) = next_link {
                let mut request = RESTRequest {
                    url: UriTemplate::new(next),
                    data: HashMap::new(),
                    method: HTTPMethod::GET,
                    schema: serde_json::Value::Null,
                    reference: None,
                };
                response = serde_json::from_str::<ResourceResponse<T>>(
                    &request.execute(client, &HashMap::new()).await?,
                )?;
                match response {
                    ResourceResponse::Error(e) => {
                        return Err(AzureError::Response(e))
                    }
                    ResourceResponse::Success(new_resources) => {
                        results.extend(new_resources.value);
                        next_link = new_resources.next_link;
                    }
                }
            }

            Ok(results)
        }
    }
}

pub async fn request_resource<T: DeserializeOwned>(
    client: &Client,
    resource: &str,
    api_version: &str,
) -> Result<Vec<T>> {
    paged_requests(
        client,
        UriTemplate::new(
            "https://management.azure.com/{resource}?api-version={api_version}",
        ),
        [
            (String::from("resource"), Template::parse("{{resource}}")?),
            (
                String::from("api_version"),
                Template::parse("{{api_version}}")?,
            ),
        ]
        .iter()
        .cloned()
        .collect(),
        [
            (String::from("resource"), resource.to_string()),
            (String::from("api_version"), api_version.to_string()),
        ]
        .iter()
        .cloned()
        .collect(),
    )
    .await
}

pub async fn request_resource_from_subscription<T: DeserializeOwned>(
    client: &Client,
    subscription: &SubscriptionId,
    resource: &str,
    api_version: &str,
) -> Result<Vec<T>> {
    paged_requests(
        client,
        UriTemplate::new("https://management.azure.com/subscriptions/{subscription}/{resource}?api-version={api_version}"),
        [(String::from("resource"), Template::parse("{{resource}}")?), 
            (String::from("api_version"), Template::parse("{{api_version}}")?),
            (String::from("subscription"), Template::parse("{{subscription}}")?)]
            .iter().cloned().collect(),
        [(String::from("resource"), resource.to_string()), 
            (String::from("api_version"), api_version.to_string()),
            (String::from("subscription"), subscription.to_string())]
            .iter().cloned().collect()
    ).await
}
