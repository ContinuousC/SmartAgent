/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use handlebars::Handlebars;
use serde::{Deserialize, Serialize};

use crate::soap::{SoapClient, SoapError};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PerfCounterDataRequest {
    // {id: PerfData}
    pub perfcounters: HashMap<String, Vec<PerfData>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct PerfData {
    pub id: String,
    pub instance: String,
    pub value: f64,
}

impl PerfCounterDataRequest {
    pub async fn new(
        client: &SoapClient,
        args: &HashMap<String, String>,
    ) -> Result<PerfCounterDataRequest, SoapError> {
        let template = r#"<SOAP-ENV:Body xmlns:ns1="urn:vim25">
					<ns1:QueryPerf xsi:type="ns1:QueryPerfRequestType">
						<ns1:_this type="PerformanceManager">{{perf_manager}}</ns1:_this>
						<ns1:querySpec>
							<ns1:entity type="HostSystem">{{esxhost}}</ns1:entity>
							<ns1:maxSample>{{samples}}</ns1:maxSample>
							{{counters}}
							<ns1:intervalId>20</ns1:intervalId>
						</ns1:querySpec>
					</ns1:QueryPerf>
			</SOAP-ENV:Body>"#
            .to_string();
        let body = Handlebars::new().render_template(&template, &args)?;
        let response = client.request(body).await?;

        let response: PerfCounterDataResponse =
            serde_xml_rs::from_str(&response)?;

        let counters = response
            .body
            .query_perf_response
            .returnval
            .values
            .iter()
            .fold(
                HashMap::new(),
                |mut map: HashMap<String, Vec<PerfData>>, val| {
                    map.entry(val.id.counter_id.to_string()).or_default().push(
                        PerfData {
                            id: val.id.counter_id.to_string(),
                            instance: val.id.instance.to_string(),
                            value: val.values.iter().sum::<f64>()
                                / val.values.len() as f64,
                        },
                    );
                    map
                },
            );

        // println!("{}", serde_json::to_string(&counters).unwrap());
        Ok(PerfCounterDataRequest {
            perfcounters: counters,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
struct PerfCounterDataResponse {
    body: Body,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
struct Body {
    query_perf_response: QueryPerfResponse,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct QueryPerfResponse {
    returnval: Returnval,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Returnval {
    //entity: Entity,
    // #[serde(rename = "sampleInfo")]
    // samples: Vec<SampleInfo>,
    #[serde(rename = "value")]
    values: Vec<Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Entity {
    //r#type: String,
    #[serde(rename = "$value")]
    value: String,
}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// struct SampleInfo {
//     timestamp: String,
//     interval: u16,
// }

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Value {
    // r#type: String,
    id: CounterInstanceId,
    #[serde(rename = "value")]
    values: Vec<f64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct CounterInstanceId {
    counter_id: String,
    instance: String,
}
