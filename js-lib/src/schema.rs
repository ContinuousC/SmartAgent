/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde_json::Value;
use std::collections::HashMap;

use thiserror::Error;

use agent_utils::TryGetFrom;
use dbschema::{
    DateTimeSchema, DbSchema, DoubleSchema, EnumSchema, HasSchema2, HasSchema4,
    ListSchema, OptionSchema, StringSchema, StructSchema, UnitSchema,
};
use etc::{Etc, Package, QueryMode, TableSpec};
use etc_base::MPId;
use metrics_types::{Metric, MetricsRow, Thresholded};
use rule_engine::selector::ValueSelector;
use value::Type;

pub fn load_schemas(
    pkg: Package,
    mode: QueryMode,
) -> Result<HashMap<String, DbSchema>, Error> {
    match mode {
        QueryMode::Monitoring | QueryMode::CheckMk => pkg
            .etc
            .mps
            .iter()
            .map(|(mp_id, mp)| {
                Ok((
                    format!("continuousc-metrics-{}", mp.elastic_name()),
                    mp_schema(mp_id, &pkg.etc, mode)?,
                ))
            })
            .chain(pkg.etc.mps.iter().map(|(mp_id, mp)| {
                Ok((
                    format!("omd-metrics-{}", mp.elastic_name()),
                    omd_mp_schema(mp_id, &pkg.etc, mode)?,
                ))
            }))
            .collect(),
        QueryMode::Discovery => pkg
            .etc
            .mps
            .iter()
            .map(|(mp_id, mp)| {
                Ok((
                    mp.elastic_name(),
                    OptionSchema::new(mp_schema(mp_id, &pkg.etc, mode)?).into(),
                ))
            })
            .collect(),
    }
}

pub fn mp_schema(
    mp: &MPId,
    etc: &Etc,
    mode: QueryMode,
) -> Result<DbSchema, Error> {
    let mut tables = HashMap::new();
    for check in etc.checks.values() {
        if &check.mp == mp {
            for table_id in &check.tables {
                let table = table_id.try_get_from(&etc.tables)?;
                let fields = table.fields_for_mode(mode, etc)?;
                if !fields.is_empty() {
                    if let Some(elastic_index) = &table.elastic_index {
                        tables.insert(elastic_index, table);
                    }
                }
            }
        }
    }

    match mode {
        QueryMode::Monitoring | QueryMode::CheckMk => {
            Ok(MetricsRow::<Thresholded<Value, Value>>::schema2(
                UnitSchema::new().into(), /* TODO: grouping schema */
                tables
                    .iter()
                    .try_fold::<_, _, Result<_, Error>>(
                        EnumSchema::new(),
                        |schema, (elastic_index, table)| {
                            Ok(schema.option(
                                *elastic_index,
                                table_schema(table, etc, mode)?,
                            ))
                        },
                    )?
                    .into(),
            ))
        }
        QueryMode::Discovery => Ok(tables
            .iter()
            .try_fold::<_, _, Result<_, Error>>(
                StructSchema::new(),
                |schema, (elastic_index, table)| {
                    Ok(schema.field(
                        *elastic_index,
                        OptionSchema::new(table_schema(table, etc, mode)?),
                    ))
                },
            )?
            .into()),
    }
}

pub fn omd_mp_schema(
    mp: &MPId,
    etc: &Etc,
    mode: QueryMode,
) -> Result<DbSchema, Error> {
    let mut tables = HashMap::new();
    for check in etc.checks.values() {
        if &check.mp == mp {
            for table_id in &check.tables {
                let table = table_id.try_get_from(&etc.tables)?;
                let fields = table.fields_for_mode(mode, etc)?;
                if !fields.is_empty() {
                    if let Some(elastic_index) = &table.elastic_index {
                        tables.insert(elastic_index, table);
                    }
                }
            }
        }
    }

    let init = StructSchema::new()
        .field("timestamp", DateTimeSchema::new())
        .field("host", StringSchema::new())
        .field("omd_site", StringSchema::new())
        .field("monitoring_pack", StringSchema::new())
        .field("group", StringSchema::new());

    Ok(tables
        .iter()
        .try_fold::<_, _, Result<_, Error>>(
            init,
            |schema, (elastic_index, table)| {
                Ok(schema.field(
                    *elastic_index,
                    OptionSchema::new(omd_table_schema(table, etc)?),
                ))
            },
        )?
        .into())
}

fn table_schema(
    table: &TableSpec,
    etc: &Etc,
    mode: QueryMode,
) -> Result<DbSchema, Error> {
    let mut fields = HashMap::new();
    for (_field_id, field) in table.fields_for_mode(mode, etc)? {
        if let Some(elastic_field) = &field.elastic_field {
            fields.insert(elastic_field, field);
        }
    }

    let fields_schema = fields.iter().try_fold::<_, _, Result<_, Error>>(
        StructSchema::new(),
        |schema, (elastic_field, field)| {
            Ok(schema.field(
                *elastic_field,
                match mode {
                    QueryMode::Monitoring | QueryMode::CheckMk => {
                        let abs = field.input_type.dbschema();
                        let rel = match &field.input_type {
                            Type::Integer | Type::Float | Type::Quantity(_) => {
                                DoubleSchema::new().into()
                            }
                            _ => EnumSchema::new().into(),
                        };
                        let (thabs, threl) = ValueSelector::metric_dbschema_for(
                            &field.input_type,
                        );
                        Metric::<Thresholded<Value, Value>>::schema4(
                            abs, thabs, rel, threl,
                        )
                    }
                    QueryMode::Discovery => OptionSchema::new(
                        EnumSchema::new()
                            .option("Ok", field.input_type.dbschema())
                            .option("Err", StringSchema::new()),
                    )
                    .into(),
                },
            ))
        },
    )?;

    match mode {
        QueryMode::Monitoring | QueryMode::CheckMk => Ok(fields_schema.into()),
        QueryMode::Discovery => Ok(EnumSchema::new()
            .option(
                "Ok",
                StructSchema::new()
                    .field("value", ListSchema::new(fields_schema))
                    .field(
                        "warnings",
                        ListSchema::new(
                            StructSchema::new()
                                .field(
                                    "verbosity",
                                    EnumSchema::new()
                                        .option("warning", UnitSchema::new())
                                        .option("info", UnitSchema::new())
                                        .option("debug", UnitSchema::new())
                                        .tag_string(),
                                )
                                .field("message", StringSchema::new()),
                        ),
                    ),
            )
            .option("Err", StringSchema::new())
            .into()),
    }
}

fn omd_table_schema(table: &TableSpec, etc: &Etc) -> Result<DbSchema, Error> {
    let mut fields = HashMap::new();
    for (_field_id, field) in
        table.fields_for_mode(QueryMode::Monitoring, etc)?
    {
        if let Some(elastic_field) = &field.elastic_field {
            fields.insert(elastic_field, field);
        }
    }

    Ok(fields
        .iter()
        .try_fold::<_, _, Result<_, Error>>(
            StructSchema::new(),
            |mut schema, (elastic_field, field)| {
                schema =
                    schema.field(*elastic_field, field.input_type.dbschema());
                if let Type::Integer | Type::Float | Type::Quantity(_) =
                    &field.input_type
                {
                    schema = schema.field(
                        format!("{}_rel", elastic_field),
                        DoubleSchema::new(),
                    );
                }
                Ok(schema)
            },
        )?
        .into())
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to decode package: {0}")]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Utils(#[from] agent_utils::Error),
    #[error(transparent)]
    Etc(#[from] etc::Error),
}
