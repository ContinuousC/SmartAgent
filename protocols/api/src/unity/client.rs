/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::HashMap, fmt::Display, ops::Deref, sync::Arc, time::Instant,
};

use agent_utils::KeyVault;
use chrono::{DateTime, Utc};
use itertools::Itertools;
use log::{debug, error, info, trace};
use reqwest::header::{self, HeaderName, HeaderValue};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tap::{Pipe, Tap, TapFallible};

use protocol::{
    auth::{BasicAuth, LookupKeyvault},
    CounterDb,
};
use tokio::sync::OnceCell;
use value::{Data, DataError, IntEnumValue, Value};

use crate::input::{FieldSpec, ParameterType, ValueTypes};

use super::{Config, DTEResult, DTError, Error, Result};

#[derive(Debug)]
pub struct Client {
    inner: reqwest::Client,
    base_url: String,
    auth: BasicAuth,
    metrics: OnceCell<DTEResult<HashMap<String, Metric>>>,
}

impl Client {
    pub async fn new(config: &Config, keyvault: KeyVault) -> Result<Self> {
        let base_url =
            format!("{}/{}", config.http.base_url(None).await?, "api");

        let auth = config.auth.lookup_keyvault(keyvault).await?;
        let (client, _) = config
            .http
            .create_client(vec![
                // (header::ACCEPT_LANGUAGE, HeaderValue::from_static("en-US")),
                (header::ACCEPT, HeaderValue::from_static("application/json")),
                (
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                ),
                (
                    HeaderName::from_static("x-emc-rest-client"),
                    HeaderValue::from_static("true"),
                ),
            ])
            .await?;

        let new = Self {
            inner: client,
            metrics: OnceCell::new(),
            base_url,
            auth,
        };
        Ok(new)
    }

    pub async fn login(&self) -> Result<()> {
        let url = format!("{}/types/basicSystemInfo/instances", self.base_url);
        debug!("login in with url: {url}");
        self.inner
            .get(url)
            .basic_auth(&self.auth.username, self.auth.password.as_deref())
            .send()
            .await
            .tap_err(|e| error!("could not send login request: {e:?}"))
            .map_err(Error::SendRequest)?
            .error_for_status()
            .tap_ok(|_| info!("login successfull"))
            .tap_err(|e| error!("login failed: {e:?}"))
            .map_err(Error::FailedLogin)
            .map(drop)
    }
    pub async fn logout(&self) -> Result<()> {
        let url =
            format!("{}/types/loginSessionInfo/action/logout", self.base_url);
        self.inner
            .post(url)
            .send()
            .await
            .map_err(Error::SendRequest)?
            .error_for_status()
            .map_err(Error::FailedLogin)
            .map(drop)
    }

    async fn request(&self, endpoint: impl Display) -> DTEResult<String> {
        let url = format!("{}/{}", self.base_url, endpoint);
        let response = self
            .inner
            .get(&url)
            .basic_auth(&self.auth.username, self.auth.password.as_deref())
            .send()
            .await
            .map_err(DTError::SendRequest)?;

        let status = response.status();
        let body = response.text().await.map_err(DTError::RecieveResponse)?;
        trace!("response from endpoint {endpoint}: {body}");

        if status.is_success() {
            Ok(body)
        } else {
            Err(DTError::FailedRequest(url, status, body))
        }
    }
    pub async fn request_data<T: DeserializeOwned>(
        &self,
        endpoint: impl Display,
    ) -> DTEResult<T> {
        self.request(endpoint)
            .await?
            .as_str()
            .pipe(serde_json::from_str)
            .map_err(DTError::DeserializeResponse)
    }

    pub async fn request_resource<T: DeserializeOwned>(
        &self,
        resource: &str,
        fields: &[&str],
        filter: &str,
    ) -> DTEResult<Vec<T>> {
        let mut querybuilder = vec!["compact=true".to_string()];

        if !fields.is_empty() {
            querybuilder.push(format!("fields={}", fields.join(",")));
        }
        if !filter.is_empty() {
            querybuilder.push(format!("filter={filter}"))
        }
        if resource == "metricValue" {
            // polling interval for most sites
            querybuilder.push("per_page=15".to_string())
        }

        let query = querybuilder.join("&");
        let base_endpoint = format!("types/{resource}/instances?{query}");
        let mut response: ResourceResponse<T> =
            self.request_data(&base_endpoint).await?;
        let mut resources = response.entries;

        while let Some(next) = response
            .metadata
            .links
            .iter()
            .find_map(|l| (l.rel == "next").then_some(&l.href))
        {
            let endpoint = format!("{}{}", base_endpoint, next);
            response = self.request_data(endpoint).await?;
            resources.extend(response.entries);
        }

        resources
            .into_iter()
            .map(|re| re.content)
            .collect::<Vec<T>>()
            .pipe(Ok)
    }

    pub async fn requests_historical_metricdefs(
        &self,
    ) -> &DTEResult<HashMap<String, Metric>> {
        const FIELDS: [&str; 2] = ["path", "type"];
        self.metrics
            .get_or_init(|| async {
                let start = Instant::now();
                self.request_resource(
                    "metric",
                    &FIELDS,
                    "isHistoricalAvailable eq true",
                )
                .await
                .map(|mtx| {
                    mtx.into_iter()
                        .map(|m: Metric| (m.path.clone(), m))
                        .collect()
                })
                .tap(|_| {
                    debug!(
                        "loaded metric definitions in {}s",
                        start.elapsed().as_secs_f32()
                    )
                })
            })
            .await
    }

    pub async fn get_metricdef(&self, path: &str) -> DTEResult<&Metric> {
        self.requests_historical_metricdefs()
            .await
            .as_ref()
            .map_err(|e| {
                DTError::Custom(format!("unable to retrieve metrics: {e}"))
            })?
            .get(path)
            .ok_or_else(|| DTError::UknownMetric(path.to_string()))
    }

    pub async fn request_historical_metric(
        &self,
        path: &[&str],
        since: DateTime<Utc>,
    ) -> DTEResult<Vec<MetricValue>> {
        let path_filter = path
            .iter()
            .map(|p| format!(r#"path EQ "{p}""#))
            .join(" OR ");
        let filter = format!(
            r#"({path_filter}) AND timestamp gt "{}""#,
            since.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
        );
        self.request_resource("metricValue", &[], &filter).await
    }

    pub async fn request_pooltiers(&self) -> DTEResult<Vec<PoolTier>> {
        const FIELDS: [&str; 3] = ["id", "tiers", "name"];
        #[derive(Deserialize)]
        struct PoolResponse {
            id: String,
            name: String,
            tiers: Vec<PoolTier>,
        }

        self.request_resource::<PoolResponse>("pool", &FIELDS, "")
            .await?
            .into_iter()
            .flat_map(|mut pr| {
                pr.tiers.iter_mut().for_each(|pt| {
                    pt.pool_id.clone_from(&pr.id);
                    pt.pool_name.clone_from(&pr.name)
                });
                pr.tiers
            })
            .collect::<Vec<PoolTier>>()
            .tap(|pts| trace!("pooltiers requested: {pts:#?}"))
            .pipe(Ok)
    }

    /// returns {poolunit: (pooltier name, pool name)}
    pub async fn request_poolunit2pooltier(
        &self,
    ) -> DTEResult<HashMap<String, (String, String)>> {
        self.request_pooltiers()
            .await?
            .into_iter()
            .flat_map(|pt| {
                pt.pool_units
                    .into_iter()
                    .map(|pu| (pu.id, (pt.name.clone(), pt.pool_id.clone())))
                    .collect::<Vec<_>>()
            })
            .collect::<HashMap<String, (String, String)>>()
            .pipe(Ok)
    }

    pub async fn request_systemcapacitytiers(
        &self,
    ) -> DTEResult<Vec<SystemCapacityTier>> {
        const FIELDS: [&str; 2] = ["id", "tiers"];
        #[derive(Deserialize)]
        struct SystemCapacity {
            id: String,
            tiers: Vec<SystemCapacityTier>,
        }

        self.request_resource::<SystemCapacity>("systemCapacity", &FIELDS, "")
            .await?
            .into_iter()
            .flat_map(|mut sc| {
                sc.tiers.iter_mut().for_each(|sct| {
                    sct.system_capacity.clone_from(&sc.id);
                });
                sc.tiers
            })
            .collect::<Vec<SystemCapacityTier>>()
            .tap(|pts| trace!("systemCapacity tiers requested: {pts:#?}"))
            .pipe(Ok)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceResponse<T> {
    pub entries: Vec<ResourceEntry<T>>,
    #[serde(flatten)]
    pub metadata: MetaData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceEntry<T> {
    pub content: T,
    #[serde(flatten, default)]
    pub metadata: Option<MetaData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaData {
    #[serde(rename = "@base", default)]
    pub base: Option<String>,
    #[serde(default)]
    pub links: Vec<Link>,
    pub updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub href: String,
    pub rel: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct IdOnly {
    pub id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metric {
    // pub id: u32,
    // pub name: String,
    pub path: String,
    pub r#type: MetricTypeInt,
    // pub description: String,
    // pub is_historical_available: bool,
    // pub is_realtime_available: bool,
    // pub unit_display_string: String,
}

impl Metric {
    fn sum(
        &self,
        metrics: &[impl AsRef<MetricValue>],
    ) -> HashMap<MetricId, f64> {
        metrics.iter().map(|mv| mv.as_ref().value_view()).fold(
            HashMap::new(),
            |mut acum, elem| {
                for (k, ev) in elem {
                    acum.entry(k).and_modify(|av| *av += ev).or_insert(ev);
                }
                acum
            },
        )
    }

    fn counter(
        &self,
        field: &FieldSpec,
        metrics: Vec<impl AsRef<MetricValue>>,
        counterdb: Arc<CounterDb>,
    ) -> HashMap<MetricId, Data> {
        debug!("calculating counter for {}", self.path);
        metrics.into_iter()
            .next()
            .map(|mv| {
                let mv = mv.as_ref();
                mv.value_view()
                    .map(|(id, value)| {
                        let key = format!("{}:{}", field.parameter_name, id.counterkey());
                        let value = match field.parameter_type {
                            ParameterType::Counter => counterdb.counter(key, value as u64, mv.timestamp.into()),
                            ParameterType::Difference => counterdb.difference(key, value as u64, mv.timestamp.into()),
                            _ => Err(DataError::External(format!("expected parametertype counter or difference. got {}", field.parameter_type)))
                        };
                        (id, value)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    fn average(
        &self,
        field: &FieldSpec,
        metrics: Vec<impl AsRef<MetricValue>>,
    ) -> HashMap<MetricId, Data> {
        debug!("calculating average for {}", self.path);
        let num_metrics = metrics.len() as f64;
        let sums = self.sum(&metrics);

        sums.into_iter()
            .map(|(s, v)| {
                let avg = v / num_metrics;
                (s, match field.parameter_type {
                    ParameterType::Integer => Ok(Value::Integer(avg as i64)),
                    ParameterType::Float => Ok(Value::Float(avg)),
                    pt => Err(DataError::TypeError(format!("Type {pt} is an invalid rate (Integer or Float required)")))
                })
            })
            .collect()
    }

    pub fn aggregate(
        &self,
        field: &FieldSpec,
        metrics: Vec<impl AsRef<MetricValue>>,
        counterdb: Arc<CounterDb>,
    ) -> DTEResult<HashMap<MetricId, Data>> {
        match MetricType::try_from(self.r#type)
            .map_err(|e| DTError::Custom(format!("invalid metrictype: {e}")))?
        {
            MetricType::Counter32
            | MetricType::Counter64
            | MetricType::VirtualCounter32
            | MetricType::VirtualCounter64 => {
                Ok(self.counter(field, metrics, counterdb))
            }

            MetricType::Rate | MetricType::Fact => {
                Ok(self.average(field, metrics))
            }

            MetricType::Text => Err(DTError::TextBasedMetric),
        }
    }
}

#[derive(
    Debug, Clone, Copy, Eq, PartialOrd, Ord, PartialEq, Serialize, Deserialize,
)]
pub struct MetricTypeInt(u8);

#[derive(
    Debug, Clone, Copy, Eq, PartialOrd, Ord, PartialEq, Serialize, Deserialize,
)]
// for counters: calculate counter / difference based on latest value
// rate: return average: sum / #metrics
// fact: return average: sum / #metrics
// text: last value as string
pub enum MetricType {
    Counter32,
    Counter64,
    Rate,
    Fact,
    Text,
    VirtualCounter32,
    VirtualCounter64,
}

impl From<MetricType> for MetricTypeInt {
    fn from(value: MetricType) -> Self {
        Self(match value {
            MetricType::Counter32 => 2,
            MetricType::Counter64 => 3,
            MetricType::Rate => 4,
            MetricType::Fact => 5,
            MetricType::Text => 6,
            MetricType::VirtualCounter32 => 7,
            MetricType::VirtualCounter64 => 8,
        })
    }
}

impl TryFrom<MetricTypeInt> for MetricType {
    type Error = u8;

    fn try_from(
        value: MetricTypeInt,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(match value.0 {
            2 => MetricType::Counter32,
            3 => MetricType::Counter64,
            4 => MetricType::Rate,
            5 => MetricType::Fact,
            6 => MetricType::Text,
            7 => MetricType::VirtualCounter32,
            8 => MetricType::VirtualCounter64,
            i => return Err(i),
        })
    }
}

impl Deref for MetricTypeInt {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricValue {
    pub path: String,
    pub timestamp: DateTime<Utc>,
    pub interval: u32,
    #[serde(default)] // how it this optional???
    pub values: HashMap<String, FloatOrValueMap>,
}

impl AsRef<MetricValue> for MetricValue {
    fn as_ref(&self) -> &MetricValue {
        self
    }
}

impl MetricValue {
    pub fn value_view(&self) -> impl Iterator<Item = (MetricId, f64)> + '_ {
        self.values
            .iter()
            .flat_map(|(sp, fvm)| fvm.unpack(vec![sp.clone()]))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FloatOrValueMap {
    Float(f64),
    ValueMap(HashMap<String, FloatOrValueMap>),
}

impl FloatOrValueMap {
    pub fn unpack(&self, id: Vec<String>) -> HashMap<MetricId, f64> {
        let mut result = HashMap::with_capacity(1);
        match self {
            Self::Float(f) => {
                result.insert(MetricId(id), *f);
            }
            Self::ValueMap(vm) => {
                result.reserve(vm.len());
                for (key, fvm) in vm {
                    let current_id = id.iter().chain([key]).cloned().collect();
                    result.extend(fvm.unpack(current_id));
                }
            }
        };
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MetricId(Vec<String>);

impl MetricId {
    pub fn counterkey(&self) -> String {
        self.0.join("->")
    }

    fn get_id(&self, idx: usize) -> Data {
        self.0
            .get(idx)
            .cloned()
            .map(Value::UnicodeString)
            .ok_or(DataError::Missing)
    }
    pub fn storage_processor(&self) -> Data {
        self.get_id(0)
    }
    pub fn key(&self) -> Data {
        self.get_id(1)
    }
    pub fn name(&self) -> Data {
        self.get_id(2)
    }
}

impl Deref for MetricId {
    type Target = [String];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemCapacityTier {
    #[serde(default)]
    pub system_capacity: String,
    pub tier_type: TierTypeInt,
    pub size_free: u64,
    pub size_total: u64,
    pub size_used: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolTier {
    #[serde(skip)]
    pub pool_id: String,
    #[serde(skip)]
    pub pool_name: String,
    pub name: String,
    #[serde(default)]
    pub tier_type: TierTypeInt,

    #[serde(default)]
    pub stripe_width: StripeWidthInt,
    #[serde(default)]
    pub raid_type: RaidTypeInt,

    pub size_total: u64,
    pub size_used: u64,
    pub size_free: u64,

    #[serde(default)]
    pub size_moving_down: u64,
    #[serde(default)]
    pub size_moving_up: u64,
    #[serde(default)]
    pub size_moving_withing: u64,

    #[serde(default)]
    pub pool_units: Vec<IdOnly>,
    pub disk_count: u32,
}

#[derive(
    Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct TierTypeInt(u8);

#[derive(
    Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct RaidTypeInt(u32);

#[derive(
    Debug, Default, Clone, Copy, Eq, PartialEq, Serialize, Deserialize,
)]
pub struct StripeWidthInt(u8);

pub trait AsValue {
    fn parse_enum(&self, value: i64, choices: Option<&ValueTypes>) -> Data {
        match choices {
            None => Err(DataError::External(
                "Parametertype is enum, but no enum values provided".to_string()
            )),
            Some(ValueTypes::Integer(choices)) => {
                IntEnumValue::new(choices.clone(), value)
                    .map(Value::IntEnum)
            },
            Some(ValueTypes::String(choices)) => Err(DataError::External(
                format!("expected int-enum-choices, got string-enum-choices: {choices:?}")
            ))

        }
    }

    fn as_value(&self, choices: Option<&ValueTypes>) -> Data;
}

impl AsValue for TierTypeInt {
    fn as_value(&self, choices: Option<&ValueTypes>) -> Data {
        self.parse_enum(self.0 as i64, choices)
    }
}

impl AsValue for RaidTypeInt {
    fn as_value(&self, choices: Option<&ValueTypes>) -> Data {
        self.parse_enum(self.0 as i64, choices)
    }
}

impl AsValue for StripeWidthInt {
    fn as_value(&self, choices: Option<&ValueTypes>) -> Data {
        self.parse_enum(self.0 as i64, choices)
    }
}
