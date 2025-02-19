/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};
use std::iter::once;

use etc_base::{DataFieldId, Row};
use value::{Type, Value};

use super::error::{QueryCheckResult, QueryResult, QueryTypeError};
use super::query::QueryType;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum PreFilter {
    #[serde(rename = "all")]
    All(Vec<PreFilter>),
    #[serde(rename = "any")]
    Any(Vec<PreFilter>),
    #[serde(rename = "is")]
    Is { field: DataFieldId, value: Value },
    #[serde(rename = "is not")]
    IsNot { field: DataFieldId, value: Value },
    #[serde(rename = "in")]
    In {
        field: DataFieldId,
        values: Vec<Value>,
    },
    #[serde(rename = "not in")]
    NotIn {
        field: DataFieldId,
        values: Vec<Value>,
    },
}

impl PreFilter {
    pub fn run(&self, row: &Row) -> QueryResult<bool> {
        match self {
            PreFilter::All(cs) => {
                cs.iter().try_fold(true, |x, c| Ok(x && c.run(row)?))
            }
            PreFilter::Any(cs) => {
                cs.iter().try_fold(false, |x, c| Ok(x || c.run(row)?))
            }
            PreFilter::Is { field, value } => match row.get(field) {
                Some(Ok(val)) => Ok(prefilter_eq(field, val, value)?),
                Some(Err(_)) => Ok(false),
                None => Err(QueryTypeError::MissingField(field.clone()).into()),
            },
            PreFilter::IsNot { field, value } => match row.get(field) {
                Some(Ok(val)) => Ok(!(prefilter_eq(field, val, value)?)),
                Some(Err(_)) => Ok(true),
                None => Err(QueryTypeError::MissingField(field.clone()).into()),
            },
            PreFilter::In { field, values } => match row.get(field) {
                Some(Ok(val)) => values.iter().try_fold(false, |x, v| {
                    Ok(x || prefilter_eq(field, val, v)?)
                }),
                Some(Err(_)) => Ok(false),
                None => Err(QueryTypeError::MissingField(field.clone()).into()),
            },
            PreFilter::NotIn { field, values } => match row.get(field) {
                Some(Ok(val)) => values.iter().try_fold(true, |x, v| {
                    Ok(x && !prefilter_eq(field, val, v)?)
                }),
                Some(Err(_)) => Ok(true),
                None => Err(QueryTypeError::MissingField(field.clone()).into()),
            },
        }
    }

    pub fn check(&self, table: QueryType) -> QueryCheckResult<QueryType> {
        match self {
            PreFilter::All(cs) => cs.iter().try_fold(table, |t, c| c.check(t)),
            PreFilter::Any(cs) => {
                let res =
                    cs.iter().try_fold(table.clone(), |t, c| c.check(t))?;
                match cs.len() {
                    // restore original keys unless exactly one condition is given
                    1 => Ok(res),
                    _ => Ok(table),
                }
            }
            PreFilter::Is { field, value } => {
                let typ = table.fields.get(field).ok_or_else(|| {
                    QueryTypeError::MissingField(field.clone())
                })?;
                prefilter_check_eq(field, typ, &value.get_type())?;
                let keys = table.keys.missing(&once(field.clone()).collect());
                Ok(QueryType {
                    singleton: keys.is_empty(),
                    fields: table.fields,
                    keys,
                })
            }
            PreFilter::IsNot { field, value } => {
                let typ = table.fields.get(field).ok_or_else(|| {
                    QueryTypeError::MissingField(field.clone())
                })?;
                prefilter_check_eq(field, typ, &value.get_type())?;
                Ok(table)
            }
            PreFilter::In { field, values } => {
                let typ = table.fields.get(field).ok_or_else(|| {
                    QueryTypeError::MissingField(field.clone())
                })?;
                values.iter().try_fold((), |_, v| {
                    prefilter_check_eq(field, typ, &v.get_type())
                })?;
                match values.len() {
                    1 => {
                        let keys =
                            table.keys.missing(&once(field.clone()).collect());
                        Ok(QueryType {
                            singleton: keys.is_empty(),
                            fields: table.fields,
                            keys,
                        })
                    }
                    _ => Ok(table),
                }
            }
            PreFilter::NotIn { field, values } => {
                let typ = table.fields.get(field).ok_or_else(|| {
                    QueryTypeError::MissingField(field.clone())
                })?;
                values.iter().try_fold((), |_, v| {
                    prefilter_check_eq(field, typ, &v.get_type())
                })?;
                Ok(table)
            }
        }
    }
}

// Temporary solution as long as we cannot fill in correctly typed values in ETC.

fn prefilter_eq(
    id: &DataFieldId,
    val: &Value,
    filter: &Value,
) -> QueryResult<bool> {
    match val.get_type() == filter.get_type() {
        true => Ok(val == filter),
        false => match (val, filter) {
            (Value::BinaryString(b), Value::UnicodeString(s))
            | (Value::UnicodeString(s), Value::BinaryString(b)) => {
                Ok(b == s.as_bytes())
            }
            (Value::Enum(v), Value::UnicodeString(f)) => {
                Ok(v.get_value() == f.as_str())
            }
            (Value::IntEnum(v), Value::Integer(f)) => {
                Ok(v.get_value_int() == *f)
            }
            (Value::IntEnum(v), Value::UnicodeString(f)) => {
                Ok(v.get_value_str() == f.as_str())
            }
            _ => Err(QueryTypeError::FilterTypeError(
                id.clone(),
                val.get_type(),
                filter.get_type(),
            )
            .into()),
        },
    }
}

fn prefilter_check_eq(
    id: &DataFieldId,
    val: &Type,
    filter: &Type,
) -> QueryCheckResult<()> {
    match val == filter {
        true => Ok(()),
        false => match (val, filter) {
            (Type::UnicodeString, Type::BinaryString) => Ok(()),
            (Type::BinaryString, Type::UnicodeString) => Ok(()),
            (Type::Enum(_), Type::UnicodeString) => Ok(()),
            (Type::IntEnum(_), Type::Integer) => Ok(()),
            (Type::IntEnum(_), Type::UnicodeString) => Ok(()),
            _ => Err(QueryTypeError::FilterTypeError(
                id.clone(),
                val.clone(),
                filter.clone(),
            )),
        },
    }
}
