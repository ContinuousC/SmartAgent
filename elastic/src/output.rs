/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use chrono::{offset::Utc, SecondsFormat};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use super::error::Result;
use super::state::State;
use expression::EvalError;
use value::Value;

/* Table and field names, to be updated if
 * type and/or semantics have changed. */
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct ElasticTableName(pub String);
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct ElasticFieldName(pub String);

pub fn write_events<T: Serialize>(
    base_dir: &Path,
    table: String,
    events: Vec<&T>,
) -> Result<()> {
    fs::create_dir_all(base_dir)?;
    let state = State::load(base_dir)?;

    let path = base_dir.join(format!("{}.json", state.last_file_id));
    let new_path = base_dir.join(format!("{}.json.new", state.last_file_id));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&new_path)?;
    let mut writer = BufWriter::new(file);

    for event in events {
        serde_json::to_writer(
            writer.by_ref(),
            &json!({ "index": { "_index": table } }),
        )?;
        writer.write_all(b"\n")?;
        serde_json::to_writer(writer.by_ref(), event)?;
        writer.write_all(b"\n")?;
    }

    fs::rename(&new_path, path)?;
    Ok(())
}

pub fn write_output(
    base_dir: &Path,
    host: &str,
    site: &str,
    data: &HashMap<
        ElasticTableName,
        Vec<HashMap<ElasticFieldName, std::result::Result<Value, EvalError>>>,
    >,
) -> Result<()> {
    fs::create_dir_all(base_dir)?;
    let state = State::load(base_dir)?;

    let path = base_dir.join(format!("{}.json", state.last_file_id));
    let new_path = base_dir.join(format!("{}.json.new", state.last_file_id));
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&new_path)?;
    let mut writer = BufWriter::new(file);

    let ts = Utc::now().to_rfc3339_opts(SecondsFormat::AutoSi, true);

    for (table_id, table_data) in data {
        write_table(writer.by_ref(), &ts, host, site, table_id, table_data)?;
    }

    fs::rename(&new_path, path)?;
    Ok(())
}

pub fn write_table<W: Write>(
    mut file: W,
    ts: &str,
    host: &str,
    site: &str,
    table_name: &ElasticTableName,
    data: &Vec<
        HashMap<ElasticFieldName, std::result::Result<Value, EvalError>>,
    >,
) -> Result<()> {
    for row in data {
        serde_json::to_writer(
            file.by_ref(),
            &json!({ "index": { "_index": table_name.0 } }),
        )?;
        file.write_all(b"\n")?;
        serde_json::to_writer(
            file.by_ref(),
            &vec![
                (
                    "@timestamp",
                    serde_json::value::Value::String(ts.to_string()),
                ),
                ("host", serde_json::value::Value::String(host.to_string())),
                ("site", serde_json::value::Value::String(site.to_string())),
            ]
            .into_iter()
            .chain(row.iter().filter_map(|(field_name, field_data)| {
                match field_data {
                    Ok(Value::BinaryString(v)) => {
                        serde_json::value::to_value(v)
                            .ok()
                            .map(|v| (field_name.0.as_str(), v))
                    }
                    Ok(Value::UnicodeString(v)) => Some((
                        field_name.0.as_str(),
                        serde_json::value::Value::String(v.to_string()),
                    )),
                    Ok(Value::Integer(v)) => {
                        serde_json::Number::from_f64(*v as f64).map(|v| {
                            (
                                field_name.0.as_str(),
                                serde_json::value::Value::Number(v),
                            )
                        })
                    }
                    Ok(Value::Float(v)) => serde_json::Number::from_f64(*v)
                        .map(|v| {
                            (
                                field_name.0.as_str(),
                                serde_json::value::Value::Number(v),
                            )
                        }),
                    Ok(Value::Quantity(v)) => v
                        .normalize()
                        .ok()
                        .and_then(|v| serde_json::Number::from_f64(v.0))
                        .map(|v| {
                            (
                                field_name.0.as_str(),
                                serde_json::value::Value::Number(v),
                            )
                        }),
                    Ok(Value::Enum(v)) => Some((
                        field_name.0.as_str(),
                        serde_json::value::Value::String(
                            v.get_value().to_string(),
                        ),
                    )),
                    Ok(Value::IntEnum(v)) => Some((
                        field_name.0.as_str(),
                        serde_json::value::Value::String(
                            v.get_value_str().to_string(),
                        ),
                    )),
                    Ok(Value::Boolean(v)) => Some((
                        field_name.0.as_str(),
                        serde_json::value::Value::Bool(*v),
                    )),
                    Ok(Value::Time(v)) => Some((
                        field_name.0.as_str(),
                        serde_json::value::Value::String(
                            v.to_rfc3339_opts(SecondsFormat::AutoSi, true),
                        ),
                    )),
                    Ok(Value::Age(v)) => serde_json::Number::from_f64(
                        v.num_milliseconds() as f64 / 1000.0,
                    )
                    .map(|v| {
                        (
                            field_name.0.as_str(),
                            serde_json::value::Value::Number(v),
                        )
                    }),
                    Ok(Value::MacAddress(v)) => Some((
                        field_name.0.as_str(),
                        serde_json::value::Value::String(
                            v.iter()
                                .map(|i| format!("{:02x}", i))
                                .collect::<Vec<_>>()
                                .join(":"),
                        ),
                    )),
                    Ok(Value::Ipv4Address(v)) => Some((
                        field_name.0.as_str(),
                        serde_json::value::Value::String(
                            v.iter()
                                .map(|i| format!("{}", i))
                                .collect::<Vec<_>>()
                                .join("."),
                        ),
                    )),
                    Ok(Value::Ipv6Address(v)) => Some((
                        field_name.0.as_str(),
                        serde_json::value::Value::String(
                            v.iter()
                                .map(|i| format!("{:x}", i))
                                .collect::<Vec<_>>()
                                .join(":"),
                        ),
                    )),
                    Ok(Value::Option(_)) => None, // TODO!
                    Ok(Value::Result(_)) => None, // TODO!
                    Ok(Value::List(_)) => None,   // TODO!
                    Ok(Value::Set(_)) => None,    // TODO!
                    Ok(Value::Map(_)) => None,    // TODO!
                    Ok(Value::Tuple(_)) => None,  // TODO!
                    Ok(Value::Json(_)) => None,   // TODO!
                    Err(_) => None,
                }
            }))
            .collect::<HashMap<&str, serde_json::value::Value>>(),
        )?;
        file.write_all(b"\n")?;
    }

    Ok(())
}
