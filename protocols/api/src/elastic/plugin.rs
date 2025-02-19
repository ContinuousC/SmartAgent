/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::convert::identity;
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::SystemTime;

use agent_utils::{KeyVault, TryGetFrom};
use chrono::{DateTime, Duration, Utc};
use etc_base::{Annotated, ProtoDataFieldId, ProtoQueryMap};
use futures::{stream, StreamExt};
use itertools::Itertools;
use log::{debug, info, trace, warn};
use protocol::CounterDb;
use serde_json::Value as JsonValue;
use tap::{Pipe, Tap, TapFallible};
use value::{Data, DataError, EnumValue, IntEnumValue, Value};

use crate::elastic::api::DataTable;
use crate::error::Result as APIResult;
use crate::input::{FieldSpec, ParameterType, ValueTypes};
use crate::plugin::TableData;
use crate::{
    plugin::DataMap, APIPlugin, Error as ApiError, Input, Plugin as ProtPlugin,
};

use super::api::Request;
use super::error::{PathError, PathResult};
use super::{Config, DTEResult, DTError};

type TableKey = Rc<Option<String>>;

fn add_parents(value: &mut JsonValue) {
    fn add_parents_to(
        value: &mut JsonValue,
        parent: Option<&str>,
        grandparent: Option<&str>,
    ) {
        match value {
            JsonValue::Array(arr) => {
                for val in arr.iter_mut() {
                    add_parents_to(val, None, parent);
                }
            }
            JsonValue::Object(obj) => {
                if let Some(p) = parent {
                    obj.insert(
                        "~".to_string(),
                        JsonValue::String(p.to_string()),
                    );
                }
                if let Some(gp) = grandparent {
                    obj.insert(
                        "~~".to_string(),
                        JsonValue::String(gp.to_string()),
                    );
                }
                for (key, val) in obj.iter_mut() {
                    if key == "~" || key == "~~" {
                        continue;
                    }
                    add_parents_to(val, Some(key.as_str()), parent);
                }
            }
            _ => (),
        }
    }

    add_parents_to(value, None, None);
}

fn follow_path<'a>(
    tree: impl IntoIterator<Item = &'a JsonValue>,
    path: &str,
) -> PathResult<Vec<&'a JsonValue>> {
    if path.starts_with('@') {
        return tree
            .into_iter()
            .map(|val| {
                val.get(&path[1..]).ok_or_else(|| {
                    PathError::StepNotFound(
                        path[1..].to_string(),
                        String::new(),
                    )
                })
            })
            .collect();
    }

    let (step, next) = match path.split_once('.') {
        Some(step) => step,
        None => (path, ""),
    };
    if path.is_empty() || path == "." {
        return Ok(tree.into_iter().collect());
    }

    tree.into_iter()
        .map(|branch| {
            if step != "*" {
                let value = branch
                    .get(step)
                    .ok_or_else(|| {
                        PathError::StepNotFound(
                            step.to_string(),
                            next.to_string(),
                        )
                    })
                    .map(|v| [v])?;

                return follow_path(value, next);
            }

            match branch {
                JsonValue::Array(vec) => follow_path(vec.iter(), next),
                JsonValue::Object(map) => {
                    // we are not interested in parents at this level
                    let iter = map
                        .iter()
                        .filter_map(|(k, v)| match k.as_str() {
                            "~" | "~~" => None,
                            _ => Some(v),
                        })
                        // we need to collect here, otherwise, the compiler cannot figure out the type of iter
                        // we cannot give it ourselves due to the lifetime of jsonvalue
                        // a possible solution is to change the tree param of follow_path to &dyn Iterator
                        // however, this causes lifetime issues. this is the easiest option
                        .collect_vec();
                    follow_path(iter, next)
                }
                _ => Err(PathError::InvalidType(
                    (*branch).clone(),
                    "array or object",
                )),
            }
        })
        .collect::<PathResult<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .pipe(Ok)
}

fn get_rowkey(
    row: &JsonValue,
    tablekey: TableKey,
    datafields: Arc<HashMap<&ProtoDataFieldId, &FieldSpec>>,
) -> Option<String> {
    let parts = datafields
        .values()
        .filter(|df| df.is_key)
        .flat_map(|df| {
            Ok::<_, PathError>(if &df.parameter_header == "#" {
                vec![tablekey.as_deref()]
            } else {
                follow_path([row], &df.parameter_header)?
                    .iter()
                    .map(|v| v.as_str())
                    .collect_vec()
            })
        })
        .flatten()
        .flatten()
        .collect::<Vec<_>>();

    (!parts.is_empty())
        .then_some(parts.join("."))
}

fn collect_field(
    datafield: &FieldSpec,
    table: &JsonValue,
    rowkey: &str,
    counterdb: &CounterDb,
) -> Data {
    let value = follow_path([table], &datafield.parameter_header)
        .map_err(|e| DataError::External(e.to_string()))?
        .pop()
        .ok_or(DataError::Missing)
        // .tap_ok(|value| {
        //     trace!(
        //         "end of path for field {}: {}",
        //         datafield.parameter_name,
        //         serde_json::to_string_pretty(value).unwrap()
        //     )
        // })
        .tap_err(|e| {
            warn!("failed to find path for {}: {e}", datafield.parameter_name)
        })?;
    let parse_err = || {
        DataError::Parse(
            serde_json::to_string(value).unwrap(),
            datafield.parameter_type.to_string(),
        )
    };

    match datafield.parameter_type {
        ParameterType::Float => match value {
            JsonValue::Number(n) => {
                n.as_f64().map(Value::Float).ok_or_else(parse_err)
            }
            JsonValue::String(s) => {
                s.parse().map(Value::Float).map_err(|_| parse_err())
            }
            _ => Err(parse_err()),
        },
        ParameterType::Integer => match value {
            JsonValue::Number(n) => {
                n.as_i64().map(Value::Integer).ok_or_else(parse_err)
            }
            JsonValue::String(s) => {
                s.parse().map(Value::Integer).map_err(|_| parse_err())
            }
            _ => Err(parse_err()),
        },
        ParameterType::String => value
            .as_str()
            .map(|s| Value::UnicodeString(s.to_string()))
            .ok_or_else(parse_err),
        ParameterType::Boolean => {
            value.as_bool().map(Value::Boolean).ok_or_else(parse_err)
        }
        ParameterType::Enum => match &datafield.values {
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
        ParameterType::Time => {
            if let Ok(dt) = serde_json::from_value(value.clone()) {
                return Ok(Value::Time(dt));
            }
            if let Ok(ts) = serde_json::from_value(value.clone()) {
                return Ok(Value::Time(
                    DateTime::from_timestamp(ts, 0).unwrap(),
                ));
            }

            Err(parse_err())
        }
        ParameterType::Age => value
            .as_i64()
            .map(Duration::milliseconds)
            .map(Value::Age)
            .ok_or_else(parse_err),
        ParameterType::IpAddress => value
            .as_str()
            .map(|s| {
                Ok(Value::Ipv4Address(
                    s.parse::<Ipv4Addr>().map_err(|_| parse_err())?.octets(),
                ))
            })
            .ok_or_else(parse_err)?,

        ParameterType::Counter => value
            .as_i64()
            .map(|i| {
                counterdb.counter(
                    format!("{}.{}", rowkey, datafield.parameter_name),
                    i as u64,
                    SystemTime::now(),
                )
            })
            .ok_or_else(parse_err)?,
        ParameterType::Difference => value
            .as_i64()
            .map(|i| {
                counterdb.difference(
                    format!("{}.{}", rowkey, datafield.parameter_name),
                    i as u64,
                    SystemTime::now(),
                )
            })
            .ok_or_else(parse_err)?,
    }
}

fn get_nodetables<'a>(
    table: &'a JsonValue,
    path: &str,
) -> DTEResult<Vec<(TableKey, Vec<&'a JsonValue>)>> {
    table
        .get("nodes")
        .and_then(|nodes| nodes.as_object())
        .map(|nodes| nodes.values())
        .into_iter()
        .flatten()
        .filter(|node| node.is_object())
        .map(|node| {
            let nodename = node
                .as_object()
                .unwrap()
                .get("name")
                .and_then(|name| name.as_str())
                .map(|s| s.to_string());

            let table =
                follow_path([node], path).map_err(DTError::PathError)?;

            Ok((Rc::new(nodename), table))
        })
        .collect::<DTEResult<Vec<_>>>()
}

fn get_clustertables<'a>(
    table: &'a JsonValue,
    path: &str,
) -> DTEResult<Vec<(TableKey, Vec<&'a JsonValue>)>> {
    let clustername = table
        .as_object()
        .and_then(|obj| obj.get("cluster_name"))
        .and_then(|name| name.as_str())
        .map(|s| s.to_string());

    let table = follow_path([table], path).map_err(DTError::PathError)?;

    Ok(vec![(Rc::new(clustername), table)])
}

// TODO: refactor to make it more readable?
fn collect_table(
    datatable: DataTable<'_>,
    data: &mut HashMap<&str, DTEResult<JsonValue>>,
    counterdb: &CounterDb,
) -> TableData {
    let table = data
        .get_mut(datatable.spec.command_name.as_str())
        .unwrap()
        .as_mut()
        .map_err(|e| DTError::Custom(e.to_string()))?;

    let tables = match datatable.spec.command_name.as_str() {
        "_nodes/stats" => get_nodetables(table, &datatable.spec.command_line),
        _ => get_clustertables(table, &datatable.spec.command_line),
    }?;
    // trace!(
    //     "end of path for tables {}:{}: {}",
    //     datatable.spec.command_name,
    //     datatable.spec.command_line,
    //     serde_json::to_string(&tables).unwrap()
    // );

    let mut idx = 0;
    let rows = tables
        .into_iter()
        .flat_map(|(tablekey, tables)| {
            tables.into_iter().map({
                let datafields = datatable.fields.clone();
                let tablekey = tablekey.clone();
                move |table| {
                    idx += 1;
                    let rowkey = get_rowkey(table, tablekey.clone(), datafields.clone())
                        .unwrap_or(idx.to_string());


                    datafields
                        .iter()
                        .map({
                            let value = tablekey.clone();
                            move |(&dfid, &df)| {
                                let field = if df.parameter_header == "#" {
                                    (*value)
                                        .clone()
                                        .map(Value::UnicodeString)
                                        .ok_or(DataError::Missing)
                                } else {
                                    collect_field(df, table, &rowkey, counterdb)
                                };

                                // trace!("data for {dfid:?}: {field:?}");
                                (dfid.clone(), field)
                            }
                        })
                        .collect()
                }
            })
        })
        .collect();

    Ok(Annotated {
        value: rows,
        warnings: Vec::new(),
    })
}

pub struct Plugin {
    key_vault: KeyVault,
    cache_dir: PathBuf,
    config: Config,
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
        }
    }

    async fn create_counterdb(&self) -> CounterDb {
        let location = self.cache_dir.join("elastic_counters.json");
        let mut counters = CounterDb::new(location);
        if let Err(e) = counters.try_load().await {
            warn!("failed to load counterdb: {e}");
        }
        counters
    }
}

#[async_trait::async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        info!("Using Elastic plugin");
        // trace!("with config: {:#?}", self.config);

        let client = self.config.get_client().await?;
        let auth = self.config.get_credentials(self.key_vault.clone()).await?;
        let base_url = self.config.http.base_url(None).await?;
        debug!("connecting to {base_url}");

        let datatables = query
            .iter()
            .map(|(dtid, dfids)| {
                let dt = ProtPlugin::get_datatable_id(dtid)
                    .try_get_from(&input.data_tables)?;
                let dfs = dfids
                    .iter()
                    .map(|dfid| {
                        Ok((
                            dfid,
                            ProtPlugin::get_datafield_id(dfid)
                                .try_get_from(&input.data_fields)
                                .map_err(ApiError::AgentUtils)?,
                        ))
                    })
                    .collect::<APIResult<_>>()?;

                Ok::<_, ApiError>(DataTable {
                    id: dtid,
                    spec: dt,
                    fields: Arc::new(dfs),
                })
            })
            .collect::<APIResult<Vec<_>>>()?;

        let apicalls = datatables.iter().fold(
            HashMap::<&str, Request>::new(),
            |mut accum: HashMap<&str, Request>, elem: &DataTable| {
                accum
                    .entry(&elem.spec.command_name)
                    // see api.rs line 40
                    // .and_modify(|req| {
                    //     let stat = elem.spec
                    //         .command_line
                    //         .split_once('.')
                    //         .map(|(stat, _)| stat)
                    //         .unwrap_or(&elem.spec.command_line);
                    //     if !(stat.is_empty() || stat == "*" || stat == ".") {
                    //         req.stats.push(stat);
                    //     }
                    // })
                    .or_insert(Request {
                        auth: &auth,
                        client: client.clone(),
                        base_url: &base_url,
                        endpoint: &elem.spec.command_name,
                        // see api.rs line 40
                        // stats: [
                        //     elem.spec
                        //         .command_line
                        //         .split_once('.')
                        //         .map(|(stat, _)| stat)
                        //         .unwrap_or(&elem.spec.command_line)
                        // ].into()
                    });
                accum
            },
        );
        info!("scheduled {} api calls", apicalls.len());

        // cannot use a closure due to lifetime issues
        async fn exec_request<'a>(
            request: &Request<'a>,
        ) -> (&'a str, DTEResult<JsonValue>) {
            info!("requesting: {}", &request.endpoint);
            let now = SystemTime::now();
            let mut data = request
                .call()
                .await
                .tap(|_| {
                    let end = now.elapsed().unwrap().as_millis();
                    info!("request to {} done in {end} ms", request.endpoint)
                })
                .tap_ok(|v| trace!("result from {}: {}", &request.endpoint, v))
                .tap_err(|e| {
                    warn!("request to {} failed: {:?}", &request.endpoint, e)
                });

            if let Ok(data) = data.as_mut() {
                add_parents(data);
            }

            (request.endpoint, data)
        }
        let mut data = apicalls
            .values()
            .map(exec_request)
            .pipe(stream::iter)
            .buffer_unordered(apicalls.len())
            // .buffer_unordered(1)
            .collect::<HashMap<_, _>>()
            .await;

        let counterdb = self.create_counterdb().await;
        let data = datatables
            .into_iter()
            .map(|datatable| {
                (
                    datatable.id.clone(),
                    collect_table(datatable, &mut data, &counterdb),
                )
            })
            .collect();
        if let Err(e) = counterdb.save().await {
            warn!("failed to save counterdb: {e}");
        }

        Ok(data)
    }
}
