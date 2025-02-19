/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::{Debug, Display},
    sync::Arc,
};

use chrono::{DateTime, Duration, Utc};
use value::{Data, DataError, Value};

use crate::{
    Config, ConnectionString, DTEResult, FieldSpec, InstanceType, Result,
    SqlDataType, Table, TableSpec,
};

pub mod mssql;
pub mod odbc;
pub mod oracle;

#[async_trait::async_trait]
pub trait SqlPlugin: Debug + Display + Sync + Send {
    fn name(&self) -> &'static str;
    async fn connection_string_per_instance(
        &self,
        base: ConnectionString,
        config: Arc<Config>,
    ) -> Result<HashMap<InstanceType, ConnectionString>>;
    fn construct_query(
        &self,
        datatable: &TableSpec,
        datafields: HashSet<&FieldSpec>,
    ) -> DTEResult<String>;
    fn transform_table<'a>(
        &self,
        spec: &TableSpec,
        table: &'a Table,
    ) -> DTEResult<Cow<'a, Table>>;

    async fn save_counters(&self) -> Result<()>;
    fn parse_counter(
        &self,
        row: &mut HashMap<String, String>,
        field: &FieldSpec,
        base_key: &str,
    ) -> Data;
    fn parse_difference(
        &self,
        row: &mut HashMap<String, String>,
        field: &FieldSpec,
        base_key: &str,
    ) -> Data;

    fn parse_value(
        &self,
        row: &mut HashMap<String, String>,
        field: &FieldSpec,
        base_key: &str,
    ) -> Data {
        if field.counter_type.is_some() {
            return self.parse_counter(row, field, base_key);
        }
        if matches!(field.parameter_type, SqlDataType::Counter) {
            return self.parse_counter(row, field, base_key);
        }
        if matches!(field.parameter_type, SqlDataType::Difference) {
            return self.parse_difference(row, field, base_key);
        }

        let val = row.remove(&field.column_name).ok_or(DataError::Missing)?;

        match &field.parameter_type {
            SqlDataType::String => Ok(Value::UnicodeString(val.to_string())),
            SqlDataType::Enum => field.values
                .as_ref()
                .map(|vt| vt.parse_val(&val))
                .ok_or_else(|| DataError::InvalidChoice("No choices set for this enum".to_string()))?,
            SqlDataType::Integer => val.trim().parse()
                .map(Value::Integer)
                .map_err(|e| DataError::TypeError(format!("Cannot parse {val} to an integer: {e}"))),
            SqlDataType::Float => val.trim().parse()
                .map(Value::Float)
                .map_err(|e| DataError::TypeError(format!("Cannot parse {val} to a float: {e}"))),
            SqlDataType::Bool => match val.as_str() {
                "0" => Ok(Value::Boolean(false)),
                "1" => Ok(Value::Boolean(true)),
                _ => Err(DataError::TypeError(format!("Cannot parse {val} to a boolean")))
            },
            SqlDataType::DateTime => DateTime::parse_from_rfc3339(&val)
                .map(|dt| Value::Time(dt.with_timezone(&Utc)))
                .map_err(|e| DataError::TypeError(format!("Cannot parse {val} to a datetime (rfc3339 required): {e}"))),
            SqlDataType::Age => val.parse()
                .map_err(|e| DataError::TypeError(format!("Cannot parse {val} to an integer: {e}")))
                .map(|secs| Value::Age(Duration::seconds(secs))),
            SqlDataType::Binary => unimplemented!("Binary datatype not yet implemented"),
            SqlDataType::Counter | SqlDataType::Difference => unreachable!()
        }
    }
}
