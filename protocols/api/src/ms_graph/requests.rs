/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::ms_graph::error::Result;
use crate::ms_graph::plugin::{request_with_retry, MSGRAPH_ENDPOINT};
use crate::ms_graph::ResourceResponse;

pub async fn get_object<T: DeserializeOwned>(
    client: &Client,
    endpoint: &str,
) -> Result<Vec<T>> {
    let url = format!("{}/{}", MSGRAPH_ENDPOINT, endpoint);
    let response = request_with_retry(client, &url, 3)
        .await
        .map_err(|e| e.to_err())?;
    Ok(response.json::<ResourceResponse<T>>().await?.value)
}
