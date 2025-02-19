/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use query::QueryWarning;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;

use dbschema::Timestamped;
use metrics_types::{
    AggregatedStatus, ByEventCategory, Data, Grouping, ItemTypeId, Metric,
    Metrics, MetricsError, MetricsInfo, MetricsResult, MetricsSuccess,
    MetricsTable,
};

use agent_utils::TryGetFrom;
use etc::{FieldSpec, QueryMode, Spec, TableSpec};
use etc_base::{Annotated, FieldId, MPId, Protocol, TableId, Warning};
use expression::{EvalCell, EvalResult, Expr};
use protocol::PluginManager;

use super::super::error::{Error, Result};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CheckTask {
    host_id: String,
    mp_id: MPId,
    table_ids: Option<HashSet<TableId>>,
    config: HashMap<Protocol, Value>,
}

impl Eq for CheckTask {}

impl PartialEq for CheckTask {
    fn eq(&self, other: &Self) -> bool {
        self.host_id == other.host_id
            && self.mp_id == other.mp_id
            && self.table_ids == other.table_ids
        //&& self.config == other.config
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct CheckKey(String, MPId);

impl CheckTask {
    pub fn key(&self) -> CheckKey {
        CheckKey(self.host_id.clone(), self.mp_id.clone())
    }

    pub async fn run(
        &self,
        plugin_manager: &PluginManager,
        spec: &Spec,
        data_sender: &mpsc::Sender<(
            String,
            String,
            Timestamped<MetricsTable<Data<Value>>>,
        )>,
    ) -> Result<()> {
        let now = Utc::now();

        let mp = self.mp_id.try_get_from(&spec.etc.mps)?;
        let mp_tables_iter = spec
            .etc
            .checks
            .values()
            .filter(|check| check.mp == self.mp_id)
            .flat_map(|check| {
                check
                    .tables
                    .iter()
                    .filter(|table_id| {
                        match table_id.try_get_from(&spec.etc.tables) {
                            Ok(table) => table.monitoring,
                            Err(_) => false,
                        }
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            });

        let table_ids: HashSet<TableId> = match &self.table_ids {
            Some(filter_ids) => mp_tables_iter
                .filter(|table_id| filter_ids.contains(table_id))
                .collect(),
            None => mp_tables_iter.collect(),
        };

        let prot_queries =
            spec.queries_for(&table_ids, QueryMode::Monitoring)?;

        log::debug!("Running queries: {:?}", prot_queries);

        let data = plugin_manager
            .run_queries(
                &spec.input,
                self.config
                    .iter()
                    .map(|(k, v)| {
                        Ok((
                            k.clone(),
                            serde_json::value::to_raw_value(&v)
                                .map_err(Error::ConfigToRaw)?,
                        ))
                    })
                    .collect::<Result<_>>()?,
                &prot_queries,
            )
            .await?;

        let tables = table_ids
            .iter()
            .map(|table_id| {
                Ok((
                    table_id.clone(),
                    table_id.try_get_from(&spec.etc.tables)?.calculate(
                        QueryMode::Monitoring,
                        &spec.etc,
                        &data,
                    )?,
                ))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        for (table_id, res) in tables.into_iter() {
            let table = table_id.try_get_from(&spec.etc.tables)?;
            let elastic_index = match &table.elastic_index {
                Some(es_index) => es_index.to_string(),
                None => continue, /* No place to save warning :( */
            };
            let item_type = table
                .item_type
                .as_ref()
                .map(|(s, _)| s.as_ref())
                .unwrap_or("unknown")
                .to_string();

            let result = match res {
                Ok(query_result) => {
                    build_table_result(spec, table, &item_type, query_result)?
                }
                Err(e) => MetricsResult::Error(MetricsError {
                    message: e.to_string(),
                }),
            };

            let table_metrics = MetricsTable {
                queried_item_type: ItemTypeId::from(
                    match mp.elastic_name().split('-').next() {
                        Some("azure") => {
                            "MP/builtin/azure_resource_group".to_string()
                        }
                        Some("office365") => {
                            "MP/builtin/azure_tenant".to_string()
                        }
                        _ => "MP/builtin/host".to_string(),
                    },
                ),
                queried_item_id: self.host_id.to_string(),
                item_type: ItemTypeId::from(format!(
                    "MP/{}/{}",
                    mp.elastic_name(),
                    elastic_index
                )),
                result,
            };

            match &table_metrics.result {
                MetricsResult::Success(_) => {
                    log::debug!("Sending data (success)...");
                }
                MetricsResult::Error(MetricsError { message }) => {
                    log::debug!("Sending data (failed: {})...", message);
                }
            }

            let e = data_sender
                .send((
                    mp.elastic_name(),
                    elastic_index,
                    Timestamped {
                        timestamp: now,
                        value: table_metrics,
                    },
                ))
                .await;

            match e.is_ok() {
                true => log::debug!("Data successfully queued "),
                false => log::debug!("Failed to queue data"),
            }
            e?
        }

        Ok(())
    }
}

fn build_table_result(
    spec: &Spec,
    table: &TableSpec,
    item_type: &str,
    Annotated {
        value: rows,
        warnings,
    }: Annotated<Vec<HashMap<FieldId, EvalResult>>, QueryWarning>,
) -> Result<MetricsResult<Data<Value>>> {
    let monitoring_fields = table.monitoring_fields(&spec.etc)?;
    let mut warnings = warnings
        .into_iter()
        .map(|Warning { verbosity, message }| {
            format!("{}: {}", verbosity, message)
        })
        .collect::<Vec<_>>();
    let table_metrics = rows
        .iter()
        .enumerate()
        .filter_map(|(i, row)| {
            build_row_result(
                spec,
                table,
                item_type,
                monitoring_fields.as_slice(),
                i,
                row,
                &mut warnings,
            )
        })
        .collect();

    Ok(MetricsResult::Success(MetricsSuccess {
        info: MetricsInfo {
            status: AggregatedStatus::default(),
            subtable_status: HashMap::new(),
            inventory_status: None,
            status_by_category: ByEventCategory::default(),
            warnings,
        },
        metrics: table_metrics,
    }))
}

fn build_row_result(
    spec: &Spec,
    table: &TableSpec,
    item_type: &str,
    monitoring_fields: &[(&FieldId, &FieldSpec)],
    i: usize,
    row: &HashMap<FieldId, EvalResult>,
    warnings: &mut Vec<String>,
) -> Option<Metrics<Data<Value>>> {
    let eval_row = row
        .iter()
        .map(|(field_id, val)| {
            let field = field_id.try_get_from(&spec.etc.fields).unwrap(); // TODO: remove unwrap!!!
            (field.name.as_ref(), EvalCell::new_evaluated(val.clone()))
        })
        .collect();

    let item_id = match &table.item_id {
        None => Some(item_type.to_string()), /* only possible for items? */
        Some(expr) => match expr
            .eval_in_row(Some(&eval_row), None)
            .and_then(|id| Ok(id.into_string()?))
        {
            Ok(id) => Some(id),
            Err(e) => {
                warnings.push(format!(
                    "Warning: failed to calculate item_id for row {}: {}",
                    i, e
                ));
                None
            }
        },
    }?;

    let item_metrics = monitoring_fields
        .iter()
        .filter_map(|(field_id, field)| {
            build_field_result(row, &eval_row, field_id, field)
        })
        .collect();

    Some(Metrics {
        entity_id: None,
        grouping: Grouping::Item(item_id),
        metrics: item_metrics,
        status: None,
        status_by_category: ByEventCategory::default(),
    })
}

fn build_field_result<'a>(
    row: &'a HashMap<FieldId, EvalResult>,
    eval_row: &'a HashMap<
        &'a str,
        EvalCell<
            'a,
            std::result::Result<value::Value, value::DataError>,
            value::Value,
        >,
    >,
    field_id: &'a FieldId,
    field: &'a FieldSpec,
) -> Option<(String, Metric<Data<Value>>)> {
    let elastic_field = match &field.elastic_field {
        Some(es_field) => es_field.to_string(),
        None => {
            //warnings.push(format!("missing ElasticField property for {}", &field.name));
            return None;
        }
    };
    let value = row.get(field_id).map_or_else(
        || Err("missing value".to_string()),
        |r| r.as_ref().cloned().map_err(|e| e.to_string()),
    );
    let relative = field.reference.as_ref().map(|ref_expr| {
        //let value = value.as_ref();
        let rel_expr = Expr::Div(
            Box::new(Expr::Literal(value.clone()?)),
            Box::new(ref_expr.clone()),
        );
        rel_expr
            .eval_in_row(Some(eval_row), None)
            .map_err(|e| e.to_string())
    });
    Some((
        elastic_field,
        Metric {
            value: Some(
                value.and_then(|v| v.to_json_value_unit(field.display_unit)),
            ),
            relative: relative.map(|v| {
                v.and_then(|v| {
                    v.to_json_value_unit(Some(
                        field
                            .relative_display_type
                            .unwrap_or(etc::RelativeDisplayType::Percentage)
                            .display_unit(),
                    ))
                })
            }),
            ..Metric::default()
        },
    ))
}
