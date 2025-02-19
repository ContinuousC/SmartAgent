/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use handlebars::Handlebars;
use log::debug;
use serde::{Deserialize, Serialize};
// use serde_json;

use crate::soap::{SoapClient, SoapError, Value};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PerfCounterSyntaxRequest {
    pub body: Body,
}

impl PerfCounterSyntaxRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<PerfCounterSyntaxRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
							<ns1:QueryPerfCounter xsi:type="ns1:QueryPerfCounterRequestType">
								<ns1:_this type="PerformanceManager">{{perf_manager}}</ns1:_this>
								{{{counterids}}}
							</ns1:QueryPerfCounter>
						</SOAP-ENV:Body>"#
            .to_string();
        debug!("retrieving countersyntax");
        let body = Handlebars::new().render_template(&template, &args)?;
        let response = client.request(body).await?;
        // debug!("response:\n{response}");
        debug!(
            "as jsonvalue: {}",
            serde_json::to_string(
                &serde_xml_rs::from_str::<serde_json::Value>(&response)
                    .unwrap()
            )
            .unwrap()
        );
        let response: PerfCounterSyntaxRequest =
            serde_xml_rs::from_str(&response)?;

        Ok(response)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct Body {
    pub query_perf_counter_response: QueryPerfCounterResponse,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QueryPerfCounterResponse {
    pub returnval: Vec<ReturnValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ReturnValue {
    pub key: Value<i32>,
    pub name_info: Info,
    pub group_info: Info,
    pub unit_info: Info,
    pub rollup_type: RollupType,
    pub stats_type: StatsType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Info {
    pub label: Value<Option<String>>,
    pub summary: Value<Option<String>>,
    pub key: Value<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum RollupType {
    Summation,
    Average,
    Latest,
    Maximum,
    Minimum,
    None,
    #[serde(other)]
    Other,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StatsType {
    Absolute,
    Rate,
    Delta,
    #[serde(other)]
    Other,
}
