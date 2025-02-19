/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};
// use serde_json;

use crate::soap::{SoapClient, SoapError, Value};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct AvailableCountersRequest {
    pub body: Body,
}

impl AvailableCountersRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<AvailableCountersRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
							<ns1:QueryAvailablePerfMetric xsi:type="ns1:QueryAvailablePerfMetricRequestType">
								<ns1:_this type="PerformanceManager">{{perf_manager}}</ns1:_this>
								<ns1:entity type="HostSystem">{{esxhost}}</ns1:entity>
								<ns1:intervalId>20</ns1:intervalId>
							</ns1:QueryAvailablePerfMetric>
						</SOAP-ENV:Body>"#.to_string();
        let body = Handlebars::new().render_template(&template, &args)?;
        let response = client.request(body).await?;
        let response: AvailableCountersRequest =
            serde_xml_rs::from_str(&response)?;

        Ok(response)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Body {
    pub query_available_perf_metric_response: QueryAvailablePerfMetricResponse,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryAvailablePerfMetricResponse {
    pub returnval: Vec<ReturnValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReturnValue {
    pub counter_id: Value<i32>,
    pub instance: Value<Option<String>>,
}
