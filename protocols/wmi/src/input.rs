/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::sync::Arc;

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use chrono_tz::Tz;
use log::{debug, trace};
use serde::{Deserialize, Serialize};

use agent_utils::TryAppend;
use etc_base::{DataFieldId, DataTableId};
use value::{DataError, EnumValue, IntEnumValue, Type, Value};

use crate::config::WmiQuircks;
use crate::counters::{CounterDB, WmiCounter};
use crate::error::TypeResult;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub struct Input {
    pub data_tables: HashMap<DataTableId, TableSpec>,
    pub data_fields: HashMap<DataFieldId, FieldSpec>,
    #[serde(default)] // backwards compatibility with older spec files
    pub data_table_fields: HashMap<DataTableId, HashSet<DataFieldId>>,
}

impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> agent_utils::Result<()> {
        self.data_tables.try_append(other.data_tables)?;
        self.data_fields.try_append(other.data_fields)?;
        self.data_table_fields.try_append(other.data_table_fields)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TableSpec {
    #[serde(rename = "NameSpace")]
    pub namespace: String,
    #[serde(rename = "ClassName")]
    pub classname: String,
    #[serde(rename = "InstancePlugin")]
    pub instance_plugin: Option<InstancePlugin>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct FieldSpec {
    pub property_name: String,
    pub property_type: WmiType,
    pub values: Option<ValueTypes>,
    pub is_key: bool,
    pub counter_type: Option<WmiCounter>,
    pub classname: String,
}

impl FieldSpec {
    pub fn get_type(&self) -> TypeResult<Type> {
        if let Some(valtypes) = &self.values {
            Ok(match valtypes {
                ValueTypes::Integer(values) => Type::IntEnum(values.clone()),
                ValueTypes::String(values) => Type::Enum(values.clone()),
            })
        } else {
            match self.property_type {
                WmiType::Boolean => Ok(Type::Boolean),
                WmiType::DateTime => Ok(Type::Time),

                WmiType::String => Ok(Type::UnicodeString),
                WmiType::Object => Ok(Type::UnicodeString),
                WmiType::Reference => Ok(Type::UnicodeString),

                WmiType::Float => Ok(Type::Float),
                _ => Ok(Type::Integer),
            }
        }
    }

    pub async fn parse_var(
        &self,
        wmi_obj: &HashMap<String, String>,
        counter_db: Arc<CounterDB>,
        base_key: &String,
        quircks: &WmiQuircks,
    ) -> Result<Value, DataError> {
        let v = wmi_obj.get(&self.property_name).ok_or(DataError::Missing)?;
        if let Some(valtypes) = &self.values {
            match valtypes {
                ValueTypes::Integer(vals) => {
                    let val = v.parse::<i64>().map_err(|_e| {
                        DataError::TypeError(format!(
                            "cannot parse {} ({}) to an integer(enum)",
                            &self.property_name, v
                        ))
                    })?;
                    Ok(Value::IntEnum(IntEnumValue::new(vals.clone(), val)?))
                }
                ValueTypes::String(vals) => Ok(Value::Enum(EnumValue::new(
                    vals.clone(),
                    v.to_string(),
                )?)),
            }
        } else if let Some(counter_type) = self.counter_type.clone() {
            let val = counter_type.get_wmi_counter(
                base_key,
                &self.property_name,
                counter_db,
                wmi_obj,
            );
            trace!(
                "counter value: {:?}: {:?}",
                (&base_key, &self.property_name),
                &val
            );
            val
        } else {
            match self.property_type {
                WmiType::String | WmiType::Object | WmiType::Reference => {
                    self.parse_string(v)
                }
                WmiType::Boolean => v
                    .to_lowercase()
                    .parse::<bool>()
                    .map(Value::Boolean)
                    .map_err(|_e| {
                        DataError::TypeError(format!(
                            "cannot parse {} ({}) to a boolean",
                            &self.property_name, v
                        ))
                    }),
                WmiType::Float => {
                    v.parse::<f64>().map(Value::Float).map_err(|_e| {
                        DataError::TypeError(format!(
                            "cannot parse {} ({}) to a float",
                            &self.property_name, v
                        ))
                    })
                }
                WmiType::DateTime => self.parse_dt(v, quircks),
                _ => v.parse::<i64>().map(Value::Integer).map_err(|_e| {
                    DataError::TypeError(format!(
                        "cannot parse {} ({}) to a integer",
                        &self.property_name, v
                    ))
                }),
            }
        }
    }
    fn parse_string(
        &self,
        s: &String,
    ) -> std::result::Result<Value, DataError> {
        Ok(Value::UnicodeString(s.to_string()))
    }

    fn parse_dt(
        &self,
        s: &String,
        quircks: &WmiQuircks,
    ) -> std::result::Result<Value, DataError> {
        let tz = quircks
            .get_tz(&self.classname, &self.property_name)
            .transpose()
            .map_err(|e| {
                DataError::TypeError(format!("Cannot parse timezone: {}", e))
            })?;
        self.parse_wmidt(s, &tz)
            .or_else(|_| self.parse_cimdt(s, &tz))
    }

    fn parse_wmidt(
        &self,
        s: &String,
        tz: &Option<chrono_tz::Tz>,
    ) -> std::result::Result<Value, DataError> {
        if s.len() < 21 {
            return Err(DataError::TypeError(format!(
                "cannot parse '{}' to a datetime",
                s
            )));
        }

        let (datetime_part, tz_part) = s.split_at(21);
        let offset: i32 = tz_part.parse().map_err(|_| {
            DataError::TypeError(format!("cannot parse '{}' to a datetime", s))
        })?;
        let offset = FixedOffset::east_opt(offset * 60).ok_or_else(|| {
            DataError::TypeError(format!("cannot parse '{}' to a datetime", s))
        })?;
        let dt: DateTime<Utc> = match tz {
            Some(tz) => tz
                .datetime_from_str(datetime_part, "%Y%m%d%H%M%S.%f")
                .map_err(|_| {
                    DataError::TypeError(format!(
                        "cannot parse '{}' to a datetime",
                        s
                    ))
                })?
                .with_timezone(&Utc),
            None => offset
                .datetime_from_str(datetime_part, "%Y%m%d%H%M%S.%f")
                .map_err(|_| {
                    DataError::TypeError(format!(
                        "cannot parse '{}' to a datetime",
                        s
                    ))
                })?
                .with_timezone(&Utc),
        };

        Ok(Value::Time(dt))
    }

    fn parse_cimdt(
        &self,
        s: &String,
        offset: &Option<chrono_tz::Tz>,
    ) -> std::result::Result<Value, DataError> {
        let dt: DateTime<Utc> = offset
            .unwrap_or(Tz::UCT)
            .datetime_from_str(s, "%d/%m/%Y %H:%M:%S")
            .map_err(|_| {
                DataError::TypeError(format!(
                    "cannot parse '{}' to a datetime",
                    s
                ))
            })?
            .with_timezone(&Utc);

        debug!("parsing cim to datetime: {}", &s);
        debug!(
            "to_rfc3339_opts: {}",
            dt.to_rfc3339_opts(chrono::SecondsFormat::Micros, true)
        );

        Ok(Value::Time(dt))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ValueTypes {
    Integer(
        #[serde(with = "agent_serde::arc_intkey_map")]
        Arc<BTreeMap<i64, String>>,
    ),
    String(Arc<BTreeSet<String>>),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum InstancePlugin {
    MSSQL,
}

impl fmt::Display for InstancePlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::MSSQL => "MSSQL",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WmiType {
    Integer,
    Float,

    #[serde(alias = "Uint8")]
    UInt8,
    #[serde(alias = "Uint16")]
    UInt16,
    #[serde(alias = "Uint32")]
    UInt32,
    #[serde(alias = "Uint64")]
    UInt64,
    #[serde(alias = "Sint8")]
    SInt8,
    #[serde(alias = "Sint16")]
    SInt16,
    #[serde(alias = "Sint32")]
    SInt32,
    #[serde(alias = "Sint64")]
    SInt64,

    Boolean,
    String,
    DateTime,
    Object,
    Reference,
}
