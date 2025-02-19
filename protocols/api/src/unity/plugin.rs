/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use agent_utils::{KeyVault, TryGetFrom};
use chrono::{DateTime, Duration, Utc};
use etc_base::{Annotated, ProtoDataFieldId, ProtoDataTableId, ProtoQueryMap};
use futures::{stream, StreamExt};
use itertools::Itertools;
use log::{debug, error, info, trace, warn};
use protocol::CounterDb;
use tap::{Pipe, Tap, TapFallible};
use tokio::sync::OnceCell;
use value::{Data, DataError, EnumValue, IntEnumValue, Value};

use super::client::{Metric, MetricValue};
use super::{AsValue, Client, Config, Error, Result};
use crate::error::{DTError as APIDTError, Result as APIResult};
use crate::input::{FieldSpec, ParameterType, TableSpec, ValueTypes};
use crate::plugin::TableData;
use crate::unity::DTError;
use crate::{plugin::DataMap, APIPlugin, Input, Plugin as ProtPlugin};

struct DtRequest<'a> {
    dtid: &'a ProtoDataTableId,
    dt: &'a TableSpec,
    dfs: HashMap<&'a ProtoDataFieldId, &'a FieldSpec>,
}

struct MetricTable<'a, T>
where
    T: AsRef<MetricValue>,
{
    metricdef: &'a Metric,
    metrics: Vec<T>,
    counterdb: Arc<CounterDb>,
    dfs: &'a HashMap<&'a ProtoDataFieldId, &'a FieldSpec>,
}

impl<'a, T> MetricTable<'a, T>
where
    T: AsRef<MetricValue>,
{
    fn into_tabledata(self) -> TableData {
        let (&value_id, &value_spec) = self
            .dfs
            .iter()
            .find(|f| f.1.parameter_name == "value")
            .ok_or(DTError::ValuespecRequired)
            .map_err(APIDTError::Unity)?;

        let value = self
            .metricdef
            .aggregate(value_spec, self.metrics, self.counterdb.clone())?
            .into_iter()
            .map(|(mid, val)| {
                self.dfs
                    .iter()
                    .filter(|(_fid, df)| (df.parameter_header != "value"))
                    .map(|(&fid, df)| {
                        (
                            fid.clone(),
                            match df.parameter_name.as_str() {
                                "sp" => mid.storage_processor(),
                                "key" => mid.key(),
                                "name" => mid.name(),
                                ph => {
                                    warn!("unknown parameter header: {ph}");
                                    Err(DataError::Missing)
                                }
                            },
                        )
                    })
                    .chain([(value_id.clone(), val)])
                    .collect()
            })
            .inspect(|row| println!("resulting row: {row:#?}"))
            .collect();

        Ok(Annotated {
            warnings: Vec::new(),
            value,
        })
    }
}

pub struct Plugin {
    key_vault: KeyVault,
    cache_dir: PathBuf,
    config: Config,
    counterdb: OnceCell<Arc<CounterDb>>,
    timestamps: OnceCell<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl Plugin {
    pub fn new(
        cache_dir: PathBuf,
        key_vault: KeyVault,
        config: Config,
    ) -> Self {
        Self {
            key_vault,
            cache_dir,
            config,
            counterdb: OnceCell::new(),
            timestamps: OnceCell::new(),
        }
    }

    async fn init_client(&self) -> Result<Arc<Client>> {
        let client = Client::new(&self.config, self.key_vault.clone()).await?;
        // let auth = self.config.auth.lookup_keyvault(self.key_vault.clone()).await?;
        let start = Instant::now();
        client.login().await?;
        debug!("login in took: {}s", start.elapsed().as_secs_f32());
        Ok(Arc::new(client))
    }

    fn counter_file(&self) -> PathBuf {
        self.cache_dir.join("counters.json")
    }
    async fn get_counterdb(&self) -> Arc<CounterDb> {
        self.counterdb
            .get_or_init(|| async {
                let start = Instant::now();
                CounterDb::load(self.counter_file())
                    .await
                    .tap(|_| {
                        debug!(
                            "loading counterdb took: {}s",
                            start.elapsed().as_secs_f32()
                        )
                    })
                    .unwrap_or_else(|e| {
                        warn!("loading counterdb failed: {e}");
                        CounterDb::new(self.counter_file())
                    })
                    .pipe(Arc::new)
            })
            .await
            .clone()
    }

    fn timestamp_file(&self) -> PathBuf {
        self.cache_dir.join("timestamps.json")
    }
    async fn get_metric_timestamps(
        &self,
    ) -> &RwLock<HashMap<String, DateTime<Utc>>> {
        self.timestamps
            .get_or_init(|| async {
                let start = Instant::now();
                let tsf = self.timestamp_file();
                debug!("loading timestamp file: {tsf:?}");
                let content = tokio::fs::read(tsf)
                    .await
                    .tap_err(|e| warn!("could not load timestampfile: {e}"))
                    .unwrap_or_default();
                serde_json::from_slice(&content)
                    .tap(|_| {
                        debug!(
                            "loading timestamps took: {}s",
                            start.elapsed().as_secs_f32()
                        )
                    })
                    .tap_err(|e| {
                        warn!("could not deserialize timestampfile: {e}")
                    })
                    .unwrap_or_default()
            })
            .await
    }
    fn default_timestamp(dt: Option<&DateTime<Utc>>) -> DateTime<Utc> {
        let now = Utc::now();
        let default = now - Duration::minutes(15);
        let min = now - Duration::minutes(15);

        dt.copied()
            .unwrap_or(default)
            .pipe(|dt| std::cmp::max(dt, min))
    }
    async fn get_metric_timestamp(&self, path: &str) -> DateTime<Utc> {
        self.get_metric_timestamps()
            .await
            .read()
            .unwrap()
            .get(path)
            .pipe(Self::default_timestamp)
    }
    async fn get_oldest_timestamp(&self) -> DateTime<Utc> {
        self.get_metric_timestamps()
            .await
            .read()
            .unwrap()
            .values()
            .min()
            .pipe(Self::default_timestamp)
    }

    #[allow(dead_code)]
    /// initializes and sets the timestamp to its value
    /// will not panic if timestamps have not been set
    async fn init_and_set_timestamp(
        &self,
        path: String,
        timestamp: DateTime<Utc>,
    ) {
        self.get_metric_timestamps()
            .await
            .write()
            .unwrap()
            .insert(path, timestamp);
    }
    /// NOTE: this function panics if timestamps have not been set
    /// (either with Self::get_metric_timestamps, or Self::get_metric_timestamps)
    /// this way it does not have to be async
    fn set_metric_timestamp(&self, path: String, timestamp: DateTime<Utc>) {
        self.timestamps
            .get()
            .unwrap()
            .write()
            .unwrap()
            .insert(path, timestamp);
    }

    async fn save_counters(&self) {
        if !self.counterdb.initialized() {
            return;
        }

        let counters = self.get_counterdb().await;
        if let Err(e) = counters.save().await {
            warn!("could not save counters: {e}");
        }
    }

    async fn save_timestamps(&self) {
        if !self.timestamps.initialized() {
            return;
        }

        let timestamps = self.get_metric_timestamps().await;
        let timestamps = serde_json::to_vec(timestamps).unwrap();
        if let Err(e) =
            tokio::fs::write(self.timestamp_file(), timestamps).await
        {
            warn!("could not save timestamps: {e}");
        }
    }

    async fn finalize(&self, client: Arc<Client>) {
        let (_, _, cl_err) = tokio::join!(
            self.save_counters(),
            self.save_timestamps(),
            client.logout()
        );

        if let Err(e) = cl_err {
            warn!("error logging out: {e}");
        }
    }

    fn parse_value(&self, df: &FieldSpec, value: &serde_json::Value) -> Data {
        let parse_err = || {
            DataError::Parse(df.parameter_type.to_string(), value.to_string())
        };

        match df.parameter_type {
            ParameterType::Float => {
                value.as_f64().map(Value::Float).ok_or_else(parse_err)
            }
            ParameterType::Integer => {
                value.as_i64().map(Value::Integer).ok_or_else(parse_err)
            }
            ParameterType::String => value
                .as_str()
                .map(|s| Value::UnicodeString(s.to_string()))
                .ok_or_else(parse_err),
            ParameterType::Boolean => {
                value.as_bool().map(Value::Boolean).ok_or_else(parse_err)
            }
            ParameterType::Enum => match &df.values {
                None => Err(DataError::External(
                    "Parametertype is enum, but no enum values provided"
                        .to_string(),
                )),
                Some(ValueTypes::Integer(choices)) => value
                    .as_i64()
                    .map(|i| IntEnumValue::new(choices.clone(), i))
                    .transpose()?
                    .map(Value::IntEnum)
                    .ok_or_else(parse_err),
                Some(ValueTypes::String(choices)) => value
                    .as_str()
                    .map(|s| EnumValue::new(choices.clone(), s.to_string()))
                    .transpose()?
                    .map(Value::Enum)
                    .ok_or_else(parse_err),
            },
            ParameterType::Time => serde_json::from_value(value.clone())
                .map(Value::Time)
                .map_err(|e| {
                    DataError::Parse(
                        df.parameter_type.to_string(),
                        format!("error deserializing ({e}): {}", value),
                    )
                }),
            _ => Err(DataError::External(format!(
                "type {} is not allowed in get_resource commands",
                df.parameter_type
            ))),
        }
    }

    async fn get_resource(
        &self,
        request: DtRequest<'_>,
        client: Arc<Client>,
    ) -> TableData {
        debug!("requesting resource: {}", &request.dt.command_line);
        let start = Instant::now();
        let data: Vec<serde_json::Value> = client
            .request_resource(
                &request.dt.command_line,
                &request
                    .dfs
                    .iter()
                    .map(|df| df.1.parameter_name.as_str())
                    .unique()
                    .collect::<Vec<_>>(),
                "",
            )
            .await
            .tap(|_| {
                debug!(
                    "retrieving {} took {}s",
                    &request.dt.command_line,
                    start.elapsed().as_secs_f32()
                )
            })?;
        debug!("found {} {} objects", data.len(), &request.dt.command_line);
        trace!("data recieved: {data:#?}");

        let data = data
            .into_iter()
            .map(|object: serde_json::Value| {
                request
                    .dfs
                    .iter()
                    .map(|(&dfid, df)| {
                        (
                            dfid.clone(),
                            object
                                .pointer(&df.parameter_header)
                                .tap(|p| {
                                    trace!(
                                        "value of {}: {:?}",
                                        &df.parameter_header,
                                        p
                                    )
                                })
                                .ok_or(DataError::Missing)
                                .and_then(|param| self.parse_value(df, param)),
                        )
                    })
                    .collect()
            })
            .collect();

        Ok(Annotated {
            value: data,
            warnings: Vec::new(),
        })
    }

    async fn get_historic_metric(
        &self,
        request: DtRequest<'_>,
        client: Arc<Client>,
    ) -> TableData {
        debug!("requesting metric: {}", &request.dt.command_line);

        let (metricdef, counterdb, metrics) = tokio::join!(
            client.get_metricdef(&request.dt.command_line),
            self.get_counterdb(),
            async {
                let since =
                    self.get_metric_timestamp(&request.dt.command_line).await;
                debug!(
                    "retrieving {} since {} ({} min ago)",
                    &request.dt.command_line,
                    since,
                    (Utc::now() - since).num_minutes()
                );

                let start = Instant::now();
                client
                    .request_historical_metric(
                        &[&request.dt.command_line],
                        since,
                    )
                    .await
                    .tap(|_| {
                        debug!(
                            "metrics recieved in {}s",
                            start.elapsed().as_secs_f32()
                        )
                    })
            }
        );

        let metrics = metrics?;
        if let Some(metric) = metrics.first() {
            self.set_metric_timestamp(
                request.dt.command_line.clone(),
                metric.timestamp,
            )
        }

        MetricTable {
            metrics,
            metricdef: metricdef?,
            counterdb: counterdb.clone(),
            dfs: &request.dfs,
        }
        .into_tabledata()
    }

    async fn get_pooltiers(
        &self,
        dfs: HashMap<&ProtoDataFieldId, &FieldSpec>,
        client: Arc<Client>,
    ) -> TableData {
        let data = client
            .request_pooltiers()
            .await?
            .into_iter()
            .map(|mut pt| {
                dfs.iter()
                    .map(|(&dfid, df)| {
                        (
                            dfid.clone(),
                            match df.parameter_header.as_str() {
                                "pool_id" => Ok(Value::UnicodeString(
                                    std::mem::take(&mut pt.pool_id),
                                )),
                                "pool_name" => Ok(Value::UnicodeString(
                                    std::mem::take(&mut pt.pool_name),
                                )),
                                "tier_type" => {
                                    pt.tier_type.as_value(df.values.as_ref())
                                }

                                "stripe_width" => {
                                    pt.stripe_width.as_value(df.values.as_ref())
                                }
                                "raid_type" => {
                                    pt.raid_type.as_value(df.values.as_ref())
                                }

                                "size_total" => {
                                    Ok(Value::Integer(pt.size_total as i64))
                                }
                                "size_used" => {
                                    Ok(Value::Integer(pt.size_used as i64))
                                }
                                "size_free" => {
                                    Ok(Value::Integer(pt.size_free as i64))
                                }

                                "size_moving_down" => Ok(Value::Integer(
                                    pt.size_moving_down as i64,
                                )),
                                "size_moving_up" => {
                                    Ok(Value::Integer(pt.size_moving_up as i64))
                                }
                                "size_moving_within" => Ok(Value::Integer(
                                    pt.size_moving_withing as i64,
                                )),

                                "disk_count" => {
                                    Ok(Value::Integer(pt.disk_count as i64))
                                }

                                _ => Err(DataError::Missing),
                            },
                        )
                    })
                    .collect()
            })
            .collect();

        Ok(Annotated {
            value: data,
            warnings: Vec::new(),
        })
    }

    async fn get_systemcapacitytiers(
        &self,
        dfs: HashMap<&ProtoDataFieldId, &FieldSpec>,
        client: Arc<Client>,
    ) -> TableData {
        let data = client
            .request_systemcapacitytiers()
            .await?
            .into_iter()
            .map(|mut sct| {
                dfs.iter()
                    .map(|(&dfid, df)| {
                        (
                            dfid.clone(),
                            match df.parameter_name.as_str() {
                                "system_capacity" => Ok(Value::UnicodeString(
                                    std::mem::take(&mut sct.system_capacity),
                                )),

                                "tierType" => {
                                    sct.tier_type.as_value(df.values.as_ref())
                                }
                                "sizeFree" => {
                                    Ok(Value::Integer(sct.size_free as i64))
                                }
                                "sizeTotal" => {
                                    Ok(Value::Integer(sct.size_total as i64))
                                }
                                "sizeUsed" => {
                                    Ok(Value::Integer(sct.size_used as i64))
                                }
                                pm => {
                                    warn!("unknown parameter header: {pm:?}");
                                    Err(DataError::Missing)
                                }
                            },
                        )
                    })
                    .collect()
            })
            .collect();

        Ok(Annotated {
            value: data,
            warnings: Vec::new(),
        })
    }

    async fn get_poolunit2pooltier(
        &self,
        dfs: HashMap<&ProtoDataFieldId, &FieldSpec>,
        client: Arc<Client>,
    ) -> TableData {
        let data = client
            .request_poolunit2pooltier()
            .await?
            .into_iter()
            .map(|(mut unit, (mut tier, mut name))| {
                dfs.iter()
                    .map(|(&dfid, df)| {
                        (
                            dfid.clone(),
                            match df.parameter_header.as_str() {
                                "pool_unit" => Ok(Value::UnicodeString(
                                    std::mem::take(&mut unit),
                                )),
                                "pool_tier" => Ok(Value::UnicodeString(
                                    std::mem::take(&mut tier),
                                )),
                                "pool_name" => Ok(Value::UnicodeString(
                                    std::mem::take(&mut name),
                                )),

                                _ => Err(DataError::Missing),
                            },
                        )
                    })
                    .collect()
            })
            .collect();

        Ok(Annotated {
            value: data,
            warnings: Vec::new(),
        })
    }

    async fn request_datatable(
        &self,
        request: DtRequest<'_>,
        client: Arc<Client>,
    ) -> TableData {
        match request.dt.command_name.as_str() {
            "get_resource" => self.get_resource(request, client).await,
            "get_specific_historical_metrics" => {
                self.get_historic_metric(request, client).await
            }
            "get_pooltiers" => self.get_pooltiers(request.dfs, client).await,
            "get_poolunit2pooltier" => {
                self.get_poolunit2pooltier(request.dfs, client).await
            }
            "get_systemcapacitytiers" => {
                self.get_systemcapacitytiers(request.dfs, client).await
            }
            _ => Err(crate::error::DTError::CommandNotFound(
                request.dt.command_name.clone(),
            )),
        }
        .tap_err(|e| error!("error while retrieving table: {e}"))
    }

    async fn request_bulk_metrics(
        &self,
        requests: Vec<DtRequest<'_>>,
        client: Arc<Client>,
    ) -> DataMap {
        if requests.is_empty() {
            return Default::default();
        }

        let fail_all = |err: &DTError| {
            requests
                .iter()
                .map(|req| {
                    (
                        req.dtid.clone(),
                        Err(APIDTError::Unity(DTError::Custom(
                            err.to_string(),
                        ))),
                    )
                })
                .collect()
        };

        let since = self.get_oldest_timestamp().await;
        let paths = requests
            .iter()
            .map(|req| req.dt.command_line.as_str())
            .collect::<Vec<_>>();
        info!(
            "requesting {} metrics in bulk since {} ({}s ago)",
            paths.len(),
            since,
            (Utc::now() - since).num_seconds()
        );
        trace!("metrics to request: {paths:#?}");

        let (metric_defs, counterdb, metrics) = tokio::join!(
            client.requests_historical_metricdefs(),
            self.get_counterdb(),
            async {
                let start = Instant::now();
                client.request_historical_metric(&paths, since).await.tap(
                    |_| {
                        debug!(
                            "bulkloading metrics took: {}s",
                            start.elapsed().as_secs_f32()
                        )
                    },
                )
            }
        );

        let metric_defs = match metric_defs {
            Ok(md) => md,
            Err(e) => return fail_all(e),
        };
        let metrics = match metrics {
            Ok(ms) => ms,
            Err(e) => return fail_all(&e),
        };

        for req in &requests {
            if let Some(m) =
                metrics.iter().find(|&m| m.path == req.dt.command_line)
            {
                self.set_metric_timestamp(m.path.clone(), m.timestamp)
            }
        }

        requests
            .iter()
            .map(|req| {
                (
                    req.dtid.clone(),
                    metric_defs
                        .get(&req.dt.command_line)
                        .ok_or_else(|| {
                            DTError::UknownMetric(req.dt.command_line.clone())
                        })
                        .map_err(APIDTError::Unity)
                        .and_then(|metricdef| {
                            MetricTable {
                                metricdef,
                                counterdb: counterdb.clone(),
                                metrics: metrics
                                    .iter()
                                    .filter(|m| m.path == req.dt.command_line)
                                    .collect_vec(),
                                dfs: &req.dfs,
                            }
                            .into_tabledata()
                        }),
                )
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        info!("using unity plugin");
        let client = self.init_client().await?;
        info!("client succesfully initialized");

        let mut resources = Vec::with_capacity(query.len());
        let mut metrics = Vec::with_capacity(query.len());
        for (dtid, dfids) in query {
            let dt = ProtPlugin::get_datatable_id(dtid)
                .try_get_from(&input.data_tables)?;
            let dfs = dfids
                .iter()
                .map(|df_id| {
                    Ok((
                        df_id,
                        ProtPlugin::get_datafield_id(df_id)
                            .try_get_from(&input.data_fields)
                            .map_err(Error::AgentUtils)?,
                    ))
                })
                .collect::<Result<HashMap<_, _>>>()?;

            let request = DtRequest { dtid, dt, dfs };
            if dt.command_name == "get_historical_metrics" {
                metrics.push(request)
            } else {
                resources.push(async {
                    (
                        dtid.clone(),
                        self.request_datatable(request, client.clone()).await,
                    )
                });
            }
        }

        let (mut metrics, resources) = tokio::join!(
            self.request_bulk_metrics(metrics, client.clone()),
            stream::iter(resources)
                .buffer_unordered(8)
                .collect::<DataMap>()
        );

        self.finalize(client).await;
        metrics.extend(resources);

        trace!("resulting datatables: {metrics:#?}");
        Ok(metrics)
    }
}
