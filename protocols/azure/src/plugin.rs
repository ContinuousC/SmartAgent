/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

/* -*- tab-width: 4 -*- */

use async_trait::async_trait;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use futures::{stream, StreamExt};
use log::{debug, info, warn};
use regex::Regex;
use reqwest::Client;
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    path::PathBuf,
};
use tokio::{fs, io::AsyncWriteExt};
use uritemplate::UriTemplate;

use agent_utils::{KeyVault, TryGetFrom};
use etc_base::{
    Annotated, AnnotatedResult, ProtoDataFieldId, ProtoDataTableId,
    ProtoQueryMap, ProtoRow, Warning,
};
use etc_base::{DataFieldId, DataTableId, Protocol};
use logger::Verbosity;
use protocol::LocalPlugin;
use protocol::{DataFieldSpec, DataTableSpec};
use rest_protocol::{
    http::HTTPMethod, input::RESTRequest, RESTError, Template,
};
use value::{DataError, Value};

use super::config::Config;
use super::error::{AzureDataError, AzureError, Result};
use super::input::{Aggregation, Input, MetricSpec};
use super::schema::{MetricValue, Metrics, Response};

type TableData = AnnotatedResult<Vec<ProtoRow>, AzureDataError, AzureError>;
pub type DataMap = HashMap<ProtoDataTableId, TableData>;
pub type DataResult = std::result::Result<AzureData, AzureDataError>;

#[derive(Debug)]
pub struct AzureData {
    datatable_id: ProtoDataTableId,
    resource_uri: String,
    resource: String,
    timestamps: HashMap<String, DateTime<Utc>>,
    // {resource_id: {aggregation, result}}
    aggregated_data: HashMap<String, HashMap<Aggregation, Option<f64>>>,
}

pub struct Plugin {
    key_vault: KeyVault,
    cache_dir: PathBuf,
}

impl Plugin {
    pub fn new(cache_dir: PathBuf, key_vault: KeyVault) -> Self {
        Self {
            key_vault,
            cache_dir,
        }
    }

    // return {name_space: [(resource_name, resource_id)]}
    pub async fn request_resources(
        &self,
        client: &Client,
        subscriptions: Vec<String>,
    ) -> Result<HashMap<String, Vec<(String, String)>>> {
        let mut resources: HashMap<String, Vec<(String, String)>> =
            HashMap::new();
        let mut request_data: HashMap<String, Template> = HashMap::new();
        request_data.insert(
            String::from("subscription"),
            Template::parse("{{subscription}}")?,
        );
        let mut request: RESTRequest = RESTRequest {
			url: UriTemplate::new("https://management.azure.com/subscriptions/{subscription}/resources?api-version=2019-10-01"),
			data: request_data,
			method: HTTPMethod::GET,
			schema: super::schema::RESOURCES.clone(),
			reference: None
		};
        info!("subscriptions: {:?}", &subscriptions);

        for subscription in subscriptions {
            let mut wato: HashMap<String, String> = HashMap::new();
            wato.insert(String::from("subscription"), subscription);
            request.url = UriTemplate::new("https://management.azure.com/subscriptions/{subscription}/resources?api-version=2019-10-01");
            let mut request_data: HashMap<String, Template> = HashMap::new();
            request_data.insert(
                String::from("subscription"),
                Template::parse("{{subscription}}")?,
            );
            request.data = request_data;

            let response: String = request.execute(client, &wato).await?;
            // validation::validate_json(&response, &request.schema)?;
            let response: serde_json::Value = serde_json::from_str(&response)?;

            // we already validated the respoinse with json schema, so we can unwrap without consequences
            for resource_val in response
                .as_object()
                .ok_or(AzureError::RESTError(RESTError::ValidationError(
                    vec![String::from("response is not an object")],
                )))?
                .get("value")
                .ok_or(AzureError::RESTError(RESTError::ValidationError(
                    vec![String::from("response has no value parameter")],
                )))?
                .as_array()
                .ok_or(AzureError::RESTError(RESTError::ValidationError(
                    vec![String::from("the value parameter is not an array")],
                )))?
            {
                let resource = resource_val.as_object().ok_or(
                    AzureError::RESTError(RESTError::ValidationError(vec![
                        String::from("resource is not an object"),
                    ])),
                )?;
                let name_space: String = String::from(resource.get("type").ok_or(AzureError::RESTError(RESTError::ValidationError(vec![String::from("resource has no parameter 'type'")])))?
						.as_str().ok_or(AzureError::RESTError(RESTError::ValidationError(vec![String::from("the parameter 'type' in resource is not a string")])))?);
                let name: String = String::from(resource.get("name").ok_or(AzureError::RESTError(RESTError::ValidationError(vec![String::from("resource has no parameter 'name'")])))?
						.as_str().ok_or(AzureError::RESTError(RESTError::ValidationError(vec![String::from("the parameter 'name' in resource is not a string")])))?);
                let id: String = String::from(resource.get("id").ok_or(AzureError::RESTError(RESTError::ValidationError(vec![String::from("resource has no parameter 'id'")])))?
						.as_str().ok_or(AzureError::RESTError(RESTError::ValidationError(vec![String::from("the parameter 'id' in resource is not a string")])))?);

                match resources.get_mut(&name_space) {
                    Some(res) => res.push((name, id)),
                    None => {
                        let mut res: Vec<(String, String)> = Vec::new();
                        res.push((name, id));
                        resources.insert(name_space, res);
                    }
                }
            }
        }
        info!("resources: {:?}", resources);
        Ok(resources)
    }

    pub fn get_resource_group(&self, resource_id: String) -> Option<String> {
        // /subscriptions/223544ac-b371-4781-bcf7-36ec8117d5e8/  NetworkWatcherRG/providers/Microsoft.Network/networkWatchers/NetworkWatcher_westeurope
        let needle = "/resourceGroups/";

        match resource_id.find(needle) {
            Some(i) => resource_id[i + needle.len()..].find('/').map(|j| {
                String::from(
                    &resource_id[i + needle.len()..i + needle.len() + j],
                )
            }),
            None => None,
        }
    }

    pub async fn request_metrics(
        &self,
        client: &Client,
        datatable_id: ProtoDataTableId,
        resource: &String,
        resource_uri: &String,
        metric_specs: &Vec<&MetricSpec>,
        timestamps: HashMap<String, DateTime<Utc>>,
        dimension: &Option<String>,
    ) -> DataResult {
        let mut aggregated_data = HashMap::new();
        let mut new_timestamps = HashMap::new();
        let metrics_to_request: HashSet<String> =
            metric_specs.iter().map(|m| m.metric_name.clone()).collect();
        let dimension_values: HashSet<String> = metric_specs
            .iter()
            .map(|m| match m.dimension_value.clone() {
                Some(d) => d,
                None => String::new(),
            })
            .collect();
        let aggregations_to_calculate: HashSet<Aggregation> =
            metric_specs.iter().map(|m| m.aggregation.clone()).collect();
        let mut aggregations_to_request: HashSet<Aggregation> = metric_specs
            .iter()
            .flat_map(|m| match &m.aggregation {
                Aggregation::Average => {
                    vec![Aggregation::Total, Aggregation::Count]
                }
                v => vec![v.clone()],
            })
            .collect();
        aggregations_to_request.insert(Aggregation::Count);

        let mut request_data: HashMap<String, Template> = HashMap::new();
        request_data.insert(
            String::from("resourceUri"),
            Template::parse("{{resourceUri}}").map_err(|e| {
                AzureDataError::TemplateError(datatable_id.clone(), e)
            })?,
        );
        request_data.insert(
            String::from("metricnames"),
            Template::parse("{{metricnames}}").map_err(|e| {
                AzureDataError::TemplateError(datatable_id.clone(), e)
            })?,
        );
        request_data.insert(
            String::from("aggregation"),
            Template::parse("{{aggregation}}").map_err(|e| {
                AzureDataError::TemplateError(datatable_id.clone(), e)
            })?,
        );
        request_data.insert(
            String::from("timespan"),
            Template::parse("{{timespan}}").map_err(|e| {
                AzureDataError::TemplateError(datatable_id.clone(), e)
            })?,
        );

        let mut wato: HashMap<String, String> = HashMap::new();
        wato.insert(String::from("resourceUri"), resource_uri.to_string());
        wato.insert(
            String::from("aggregation"),
            aggregations_to_request
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<String>>()
                .join(","),
        );

        let is_dimension: bool = match &dimension {
            Some(d) => {
                request_data.insert(
                    String::from("$filter"),
                    Template::parse("{{dimension_name}} eq '*'").map_err(
                        |e| {
                            AzureDataError::TemplateError(
                                datatable_id.clone(),
                                e,
                            )
                        },
                    )?,
                );
                wato.insert(String::from("dimension_name"), d.to_string());
                info!("requesting dimension: {}", &d);
                true
            }
            None => false,
        };
        debug!("requestdata: {:?}", &request_data);

        for metric_chunk in metrics_to_request
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<String>>()
            .chunks(20)
        {
            let min_timestamp: DateTime<Utc> = metric_chunk
                .iter()
                .filter_map(|v| timestamps.get(v))
                .fold(None, |n: Option<DateTime<Utc>>, m| {
                    n.map_or(Some(*m), |n| Some(n.min(*m)))
                })
                .unwrap_or_else(|| Utc::now() - Duration::minutes(60));

            wato.insert(
                String::from("timespan"),
                format!(
                    "{}/{}",
                    min_timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
                    (Utc::now() + Duration::minutes(1))
                        .to_rfc3339_opts(SecondsFormat::Millis, true)
                ),
            );
            wato.insert(
                String::from("metricnames"),
                metric_chunk.to_vec().join(","),
            );

            let mut request: RESTRequest = RESTRequest {
				url: UriTemplate::new("https://management.azure.com/{+resourceUri}/providers/microsoft.insights/metrics?api-version=2018-01-01{&metricnames,aggregation,timespan,$filter}"),
				data: request_data.clone(),
				method: HTTPMethod::GET,
				schema: super::schema::METRICS.clone(),
				reference: None
			};

            let response: String =
                request.execute(client, &wato).await.map_err(|e| {
                    AzureDataError::RESTError(datatable_id.clone(), e)
                })?;
            let response: Metrics = match serde_json::from_str(&response)
                .map_err(|e| {
                    AzureDataError::SerdeJsonError(datatable_id.clone(), e)
                })? {
                Response::Ok(metrics) => metrics,
                Response::Err(err) => {
                    if !err.message.contains("Valid metrics:") {
                        return Err(AzureDataError::ResponseError(
                            datatable_id.clone(),
                            format!("{}: {}", err.code, err.message),
                        ));
                    }
                    let mut new_wato: HashMap<String, String> = wato.clone();
                    new_wato.insert(
                        String::from("metricnames"),
                        err.get_error_metrics(&metrics_to_request)
                            .map_err(|e| {
                                AzureDataError::AzureData(
                                    datatable_id.clone(),
                                    e,
                                )
                            })?
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join(","),
                    );

                    request.url = UriTemplate::new("https://management.azure.com/{+resourceUri}/providers/microsoft.insights/metrics?api-version=2018-01-01{&metricnames,aggregation,timespan,$filter}");
                    match serde_json::from_str(
                        &request.execute(client, &new_wato).await.map_err(
                            |e| {
                                AzureDataError::RESTError(
                                    datatable_id.clone(),
                                    e,
                                )
                            },
                        )?,
                    )
                    .map_err(|e| {
                        AzureDataError::SerdeJsonError(datatable_id.clone(), e)
                    })? {
                        Response::Ok(metrics) => metrics,
                        Response::Err(err) => {
                            return Err(AzureDataError::ResponseError(
                                datatable_id.clone(),
                                format!("{}: {}", err.code, err.message),
                            ))
                        }
                    }
                }
            };

            for metric in &response.value {
                match metric.error_code.as_str() {
                    "Success" => (),
                    // we see an invalid series as no series. so the table does not come out and is not inventorised
                    "InvalidSeries" => {
                        return Ok(AzureData {
                            datatable_id,
                            resource_uri: resource_uri.clone(),
                            resource: resource.clone(),
                            timestamps,
                            aggregated_data: HashMap::new(),
                        })
                    }
                    _ => match &metric.error_message {
                        Some(m) => {
                            return Err(AzureDataError::ResponseError(
                                datatable_id,
                                m.to_string(),
                            ))
                        }
                        None => {
                            return Err(AzureDataError::ResponseError(
                                datatable_id.clone(),
                                format!(
                                    "API request not successful ({})",
                                    resource_uri
                                ),
                            ))
                        }
                    },
                }
            }

            if !is_dimension {
                for metric in response.value {
                    let mut metric_values: HashMap<Aggregation, Option<f64>> =
                        HashMap::new();
                    let mut timeseries: Vec<MetricValue> = Vec::new();
                    let metric_name: String = metric.name.value;
                    let mut last_timestamp =
                        timestamps.get(&metric_name).cloned();
                    debug!(
                        "timeseries for {} ({}): {:#?}",
                        &metric_name, resource, &metric.timeseries
                    );

                    for serie in
                        metric.timeseries.iter().flat_map(|ts| &ts.data)
                    {
                        if last_timestamp
                            .map_or(true, |ts| serie.timestamp >= ts)
                            && serie.has_data()
                        {
                            timeseries.push(serie.clone());
                            last_timestamp = Some(serie.timestamp);
                        }
                    }

                    for aggregation in &aggregations_to_calculate {
                        metric_values.insert(
                            aggregation.clone(),
                            aggregation.aggregate_time_series(&timeseries),
                        );
                    }

                    aggregated_data.insert(metric_name.clone(), metric_values);
                    if let Some(ts) = last_timestamp {
                        new_timestamps.insert(metric_name, ts);
                    }
                }
            } else {
                let metric = &response.value.first().ok_or(
                    AzureDataError::ResponseError(
                        datatable_id.clone(),
                        String::from("No timeseries in response"),
                    ),
                )?;
                let mut min_last_timestamp: Option<DateTime<Utc>> = None;
                let metric_name: &String = &metric.name.value;
                for series in &metric.timeseries {
                    let dimension_name: String = match series
                        .metadatavalues
                        .as_ref()
                        .ok_or(AzureDataError::ResponseError(
                            datatable_id.clone(),
                            String::from(
                                "No metadata in response with dimension",
                            ),
                        ))?
                        .first()
                    {
                        Some(m) => m.value.clone(),
                        None => String::new(),
                    };

                    if dimension_values.contains(&dimension_name) {
                        let mut metric_values: HashMap<
                            Aggregation,
                            Option<f64>,
                        > = HashMap::new();
                        let mut timeseries: Vec<MetricValue> = Vec::new();
                        let mut last_timestamp =
                            timestamps.get(&dimension_name).cloned();

                        for serie in &series.data {
                            if last_timestamp
                                .map_or(true, |ts| serie.timestamp >= ts)
                                && serie.has_data()
                            {
                                timeseries.push(serie.clone());
                                last_timestamp = Some(serie.timestamp);
                            }
                        }

                        min_last_timestamp =
                            match (min_last_timestamp, last_timestamp) {
                                (Some(mts), Some(ts)) => Some(mts.min(ts)),
                                (None, Some(ts)) => Some(ts),
                                _ => min_last_timestamp,
                            };

                        for aggregation in &aggregations_to_calculate {
                            metric_values.insert(
                                aggregation.clone(),
                                aggregation.aggregate_time_series(&timeseries),
                            );
                        }
                        let id: String =
                            format!("{}.{}", metric_name, dimension_name);
                        aggregated_data.insert(id, metric_values);
                    }
                }

                if let Some(ts) = min_last_timestamp {
                    new_timestamps.insert(metric_name.to_string(), ts);
                }
            }
        }
        let azdata = AzureData {
            datatable_id,
            resource_uri: resource_uri.clone(),
            resource: resource.clone(),
            timestamps: new_timestamps,
            aggregated_data,
        };
        debug!("result of {}: {:#?}", &resource, &azdata);
        Ok(azdata)
    }

    fn get_datatable_id(dt_id: &ProtoDataTableId) -> DataTableId {
        DataTableId(Protocol(Self::PROTOCOL.to_string()), dt_id.clone())
    }
    fn get_datafield_id(df_id: &ProtoDataFieldId) -> DataFieldId {
        DataFieldId(Protocol(Self::PROTOCOL.to_string()), df_id.clone())
    }
}

/*
This protocol plugin is used for Azure metrics
*/
#[async_trait]
impl protocol::LocalPlugin for Plugin {
    type Error = AzureError;
    type TypeError = AzureError;
    type DTError = AzureError;
    type DTWarning = AzureDataError;

    type Input = Input;
    type Config = Config;

    const PROTOCOL: &'static str = "Azure";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn show_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> Result<String> {
        let mut out = String::new();
        for (resource_id, field_ids) in query {
            let resource = Self::get_datatable_id(resource_id)
                .try_get_from(&input.data_tables)?;
            writeln!(
                out,
                "Azure: {} with metrics {}",
                &resource.name_space,
                {
                    let mut details: Vec<String> =
                        Vec::with_capacity(field_ids.len());
                    for metric_id in field_ids {
                        let metric = Self::get_datafield_id(metric_id)
                            .try_get_from(&input.data_fields)?;
                        details.push(format!(
                            "{} ({})",
                            metric.metric_name, metric.aggregation
                        ));
                    }
                    details.join(", ")
                }
            )?;
        }

        Ok(out)
    }

    fn get_tables(
        &self,
        input: &Self::Input,
    ) -> Result<HashMap<ProtoDataTableId, DataTableSpec>> {
        Ok(input
            .data_tables
            .keys()
            .map(|dt_id| {
                let datafields = input
                    .data_table_fields
                    .get(dt_id)
                    .cloned()
                    .unwrap_or_default();
                (
                    dt_id.1.clone(),
                    DataTableSpec {
                        name: dt_id.1 .0.clone(),
                        singleton: false,
                        keys: datafields
                            .iter()
                            .map(|id| (id, input.data_fields.get(id)))
                            .filter_map(|(id, field)| {
                                if let Some(field) = field {
                                    match field.is_key {
                                        true => Some(id),
                                        false => None,
                                    }
                                } else {
                                    None
                                }
                            })
                            .map(|id| id.1.clone())
                            .collect(),
                        fields: datafields.into_iter().map(|id| id.1).collect(),
                    },
                )
            })
            .collect())
    }

    fn get_fields(
        &self,
        input: &Self::Input,
    ) -> Result<HashMap<ProtoDataFieldId, DataFieldSpec>> {
        Ok(input
            .data_fields
            .iter()
            .map(|(df_id, metric_spec)| {
                (
                    df_id.1.clone(),
                    DataFieldSpec {
                        name: format!(
                            "{}.{}",
                            metric_spec.metric_name, metric_spec.aggregation
                        ),
                        input_type: metric_spec.get_type(),
                    },
                )
            })
            .collect())
    }

    async fn run_queries(
        &self,
        input: &Input,
        config: &Config,
        query: &ProtoQueryMap,
    ) -> Result<DataMap> {
        // debug!("config: {:?}", &config);
        let client: Client = config.login(Some(&self.key_vault)).await?;

        let timestamp_file =
            self.cache_dir.join(String::from("azure_timstamps.json"));
        let mut timestamp_map: HashMap<String, HashMap<String, DateTime<Utc>>> =
            match fs::read(&timestamp_file).await {
                Err(_) => HashMap::new(),
                Ok(data) => match serde_json::from_reader(data.as_slice()) {
                    Err(_) => HashMap::new(),
                    Ok(ts) => ts,
                },
            };

        // {name_space: [(resource_name, resource_id)]}
        let name_spaces: HashMap<String, Vec<(String, String)>> = self
            .request_resources(
                &client,
                config.subscriptions.clone().unwrap_or_default(),
            )
            .await?;
        debug!("Resources: {:?}", &name_spaces);
        let metrics: HashMap<&ProtoDataTableId, Vec<&MetricSpec>> = query
            .iter()
            .map(|(dt_id, df_ids)| {
                (
                    dt_id,
                    df_ids
                        .iter()
                        .map(|id| {
                            Self::get_datafield_id(id)
                                .try_get_from(&input.data_fields)
                        })
                        .filter_map(|rm| {
                            if let Ok(m) = rm {
                                if m.metric_name != "Resource"
                                    && m.metric_name != "ResourceGroup"
                                {
                                    Some(m)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                        .collect(),
                )
            })
            .collect();

        let empty_resourcelist = Vec::new();
        let mut futures: Vec<_> = Vec::new();
        for (dt_id, _) in query {
            let resource_spec = Self::get_datatable_id(dt_id)
                .try_get_from(&input.data_tables)?;
            if let Some(metrics) = metrics.get(dt_id) {
                for (resource, resource_uri) in name_spaces
                    .get(&resource_spec.name_space)
                    .unwrap_or(&empty_resourcelist)
                {
                    if let Some(configed_groups) =
                        &config.resource_groups.as_ref()
                    {
                        if self.get_resource_group(resource_uri.clone()).map_or(
                            false,
                            |resource_group| {
                                !Regex::new(configed_groups)
                                    .unwrap()
                                    .is_match(&resource_group)
                            },
                        ) {
                            continue;
                        }
                    }

                    let timestamps = timestamp_map
                        .remove(resource_uri)
                        .unwrap_or(HashMap::new());
                    debug!(
                        "scheduling request for resource: {:?}",
                        &resource_spec
                    );
                    futures.push(self.request_metrics(
                        &client,
                        dt_id.clone(),
                        resource,
                        resource_uri,
                        metrics,
                        timestamps,
                        &resource_spec.dimension_name,
                    ));
                }
            }
        }

        let responses = stream::iter(futures)
            .buffer_unordered(8)
            .collect::<Vec<DataResult>>()
            .await;
        let mut data: HashMap<ProtoDataTableId, Vec<DataResult>> =
            HashMap::new();

        for resp in responses.into_iter() {
            let dt = match &resp {
                Ok(d) => d.datatable_id.clone(),
                Err(e) => e.get_dt(),
            };
            data.entry(dt).or_insert(Vec::new()).push(resp);
        }

        let mut datamap: DataMap = HashMap::new();
        let mut new_timestamps: HashMap<
            String,
            HashMap<String, DateTime<Utc>>,
        > = HashMap::new();

        for (dt_id, df_ids) in query {
            let mut rows = Vec::new();
            let mut errors = Vec::new();

            if let Some(azd) = data.remove(dt_id) {
                for azd in azd {
                    match azd {
                        Ok(d) => {
                            new_timestamps.insert(
                                d.resource_uri.clone(),
                                d.timestamps.clone(),
                            );
                            let mut row = HashMap::new();
                            for df_id in df_ids {
                                let metric_spec = Self::get_datafield_id(df_id)
                                    .try_get_from(&input.data_fields)?;
                                let akey = if let Some(dimension) =
                                    &metric_spec.dimension_value
                                {
                                    format!(
                                        "{}.{}",
                                        metric_spec.metric_name, dimension
                                    )
                                } else {
                                    metric_spec.metric_name.to_string()
                                };

                                if metric_spec.metric_name == "ResourceGroup" {
                                    row.insert(
                                        df_id.clone(),
                                        match self.get_resource_group(
                                            d.resource_uri.clone(),
                                        ) {
                                            Some(group) => {
                                                Ok(Value::BinaryString(
                                                    group.as_bytes().to_vec(),
                                                ))
                                            }
                                            None => Err(DataError::Missing),
                                        },
                                    );
                                    continue;
                                } else if metric_spec.metric_name == "Resource"
                                {
                                    row.insert(
                                        df_id.clone(),
                                        Ok(Value::BinaryString(
                                            d.resource
                                                .clone()
                                                .as_bytes()
                                                .to_vec(),
                                        )),
                                    );
                                    continue;
                                } else if let Some(aggrs) =
                                    d.aggregated_data.get(&akey)
                                {
                                    if let Some(aggr) =
                                        aggrs.get(&metric_spec.aggregation)
                                    {
                                        row.insert(
                                            df_id.clone(),
                                            match aggr {
                                                Some(f) => {
                                                    Ok(Value::Float(*f))
                                                }
                                                None => {
                                                    warn!("Missing aggregation from aggregations ({}): {} in {:?}", &metric_spec.metric_name, &metric_spec.aggregation, &aggr);
                                                    Err(DataError::Missing)
                                                }
                                            },
                                        );
                                        continue;
                                    }
                                }
                                warn!("Missing metrickey from aggregated data: {} in {:?}", akey, &d.aggregated_data);
                                row.insert(
                                    df_id.clone(),
                                    Err(DataError::Missing),
                                );
                            }
                            rows.push(row);
                        }
                        Err(e) => {
                            errors.push(e);
                        }
                    }
                }
            }

            debug!("Inserting rows for {}: {:?}", &dt_id, &rows);
            datamap.insert(
                dt_id.clone(),
                Ok(Annotated {
                    value: rows,
                    warnings: errors
                        .into_iter()
                        .map(|e| Warning {
                            verbosity: Verbosity::Warning,
                            message: e,
                        })
                        .collect(),
                }),
            );
        }

        fs::create_dir_all(&self.cache_dir).await?;
        fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(timestamp_file)
            .await?
            .write_all(&serde_json::to_vec(&timestamp_map)?)
            .await?;

        Ok(datamap)
    }
}
