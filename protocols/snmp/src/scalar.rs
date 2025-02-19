/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use netsnmp::{ErrType, Oid, VarType};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::sync::Arc;
use value::hashable::HashableType;

use agent_utils::DBObj;
use value::{
    Data, DataError, EnumValue, HashableValue, IntEnumValue, ResultValue,
    SetValue, Type, Value,
};

use super::counters::Counters;
use super::error::{TypeError, TypeResult};
use super::input::ObjectId;

#[derive(DBObj, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct ScalarSpec {
    #[serde(deserialize_with = "deserialize_table_id")]
    pub table: Option<ObjectId>,
    pub syntax: VarType,
    pub value_list: Option<ValueList>,
    pub value_range: Option<ValueRange>,
    #[serde(default = "default_false")]
    pub error_enum: bool,
}

fn default_false() -> bool {
    false
}

// Deserialize Option<ObjectId> (for table ids). For compatibility reasons,
// "noIndex" is also accepted and converted to None.
fn deserialize_table_id<'de, D>(
    deserializer: D,
) -> Result<Option<ObjectId>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Option::deserialize(deserializer)
        .map(|id| id.and_then(ObjectId::handle_noindex))
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum ValueList {
    #[serde(with = "agent_serde::arc_intkey_map")]
    Integer(Arc<BTreeMap<i64, String>>),
    String(Arc<BTreeMap<String, String>>),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ValueRange {
    Exact(i64),
    Range { from: Option<i64>, to: Option<i64> },
}

impl ScalarSpec {
    // Should agree with get_value and get_value_from_index
    pub fn get_type(&self) -> TypeResult<Type> {
        match self.syntax {
            VarType::Boolean => Ok(Type::Boolean),
            VarType::Integer => match &self.value_list {
                Some(ValueList::Integer(vs)) => match self.error_enum {
                    false => Ok(Type::IntEnum(vs.clone())),
                    true => Ok(Type::Result(
                        Arc::new(Type::Integer),
                        Arc::new(Type::IntEnum(vs.clone())),
                    )),
                },
                Some(_) => Err(TypeError::ExpectedIntegerValueMap),
                None => Ok(Type::Integer),
            },
            VarType::Integer32 => Ok(Type::Integer),
            VarType::Integer64 => Ok(Type::Integer),
            VarType::Unsigned64 => Ok(Type::Integer),
            VarType::Gauge => Ok(Type::Integer),
            VarType::TimeTicks => Ok(Type::Float),
            VarType::Float => Ok(Type::Float),
            VarType::Double => Ok(Type::Float),
            VarType::BitStr => match &self.value_list {
                Some(ValueList::Integer(vs)) => {
                    Ok(Type::Set(Arc::new(HashableType::IntEnum(vs.clone()))))
                }
                _ => Err(TypeError::ExpectedIntegerValueMapForBitStr),
            },
            VarType::OctetStr => match &self.value_list {
                Some(ValueList::String(vs)) => {
                    Ok(Type::Enum(Arc::new(vs.values().cloned().collect())))
                }
                Some(_) => Err(TypeError::ExpectedStringValueMap),
                None => Ok(Type::BinaryString),
            },
            VarType::Counter => Ok(Type::Float),
            VarType::Counter64 => Ok(Type::Float),
            VarType::IpAddress => Ok(Type::Ipv4Address),
            VarType::MacAddress => Ok(Type::MacAddress),
            VarType::Oid => Ok(Type::UnicodeString),
            typ => Err(TypeError::UnimplementedSnmpType(typ)),
        }
    }

    /// Get value from data. To support counters, we need to already know the index
    /// and have the index map loaded from disk.

    pub fn get_value(
        &self,
        val: Result<&netsnmp::Value, &netsnmp::ErrType>,
        object: &ObjectId,
        index: &Oid,
        counters: &mut Counters,
    ) -> Option<Data> {
        match (self.syntax, val) {
            /*** Correctly typed values. ***/

            /* Boolean and numeric types. */
            (VarType::Boolean, Ok(netsnmp::Value::Boolean(v))) => {
                Some(Ok(Value::Boolean(*v)))
            }
            (VarType::Integer, Ok(netsnmp::Value::Integer(v))) => {
                Some(self.integer_enum_from_value(*v))
            }
            (VarType::Integer32, Ok(netsnmp::Value::Integer(v))) => {
                Some(Ok(Value::Integer(*v)))
            }
            (VarType::Gauge, Ok(netsnmp::Value::Gauge(v))) => {
                Some(Ok(Value::Integer(*v as i64)))
            }
            (VarType::Integer64, Ok(netsnmp::Value::Integer64(v))) => {
                Some(self.integer_enum_from_value(*v))
            }
            (VarType::Unsigned64, Ok(netsnmp::Value::Unsigned64(v))) => {
                Some(self.integer_enum_from_value(*v as i64))
            }
            (VarType::TimeTicks, Ok(netsnmp::Value::TimeTicks(v))) => {
                Some(Ok(Value::Float(*v as f64 / 100.)))
            }
            (VarType::Float, Ok(netsnmp::Value::Float(v))) => {
                Some(Ok(Value::Float(*v as f64)))
            }
            (VarType::Double, Ok(netsnmp::Value::Double(v))) => {
                Some(Ok(Value::Float(*v)))
            }

            /* String types. */
            (VarType::BitStr, Ok(netsnmp::Value::BitStr(v))) => {
                Some(self.bitstr_set_from_value(v.to_vec()))
            }
            (VarType::OctetStr, Ok(netsnmp::Value::OctetStr(v))) => {
                Some(self.string_enum_from_value(v.to_vec()))
            }

            /* Counters */
            //(VarType::Gauge, Ok(Value::Gauge(v))) => Some(counters.get_counter(*v, object, index)),
            (VarType::Counter, Ok(netsnmp::Value::Counter(v))) => {
                Some(counters.get_counter(*v, object, index))
            }
            (VarType::Counter64, Ok(netsnmp::Value::Counter64(v))) => {
                Some(counters.get_counter(*v, object, index))
            }

            /* Special types. */
            (VarType::Oid, Ok(netsnmp::Value::Oid(v))) => {
                Some(Ok(Value::UnicodeString(format!(".{}", v))))
            }
            (VarType::IpAddress, Ok(netsnmp::Value::IpAddress(v))) => {
                Some(Ok(Value::Ipv4Address([
                    (*v & 0xff) as u8,
                    (*v >> 8 & 0xff) as u8,
                    (*v >> 16 & 0xff) as u8,
                    (*v >> 24 & 0xff) as u8,
                ])))
            }
            /* MAC Address is a standard textual convention for OCTET STRING */
            (VarType::MacAddress, Ok(netsnmp::Value::OctetStr(v))) => {
                match v.as_slice().try_into() {
                    Ok(s) => Some(Ok(Value::MacAddress(s))),
                    _ => Some(Err(DataError::TypeError(format!(
                        "Invalid MAC Address: \"{:?}\"",
                        v
                    )))),
                }
            }

            /*** Implicit casts for "innocent" mismatches between MIB and reality. ***/

            /* seen for: Netscaler */
            (VarType::Counter, Ok(netsnmp::Value::Gauge(v))) => {
                Some(counters.get_counter(*v, object, index))
            }
            /* seen for: Amaron Mirth */
            (VarType::Gauge, Ok(netsnmp::Value::Counter(v))) => {
                Some(Ok(Value::Integer(*v as i64)))
            }
            /* seen for: F5 BigIP */
            (VarType::Integer32, Ok(netsnmp::Value::Gauge(v))) => {
                Some(Ok(Value::Integer(*v as i64)))
            }
            (VarType::Gauge, Ok(netsnmp::Value::Counter64(v))) => {
                Some(Ok(Value::Integer(*v as i64)))
            }
            /* seen for: Cisco UCS */
            (VarType::BitStr, Ok(netsnmp::Value::OctetStr(v))) => {
                Some(self.bitstr_set_from_value(v.to_vec()))
            }
            /* seen for: Clearpass */
            (VarType::Integer64, Ok(netsnmp::Value::Counter64(v))) => {
                Some(Ok(Value::Integer(*v as i64)))
            }

            /*** Type mismatch. ***/
            (syn, Ok(act)) => Some(Err(DataError::TypeError(format!(
                "Type mismatch: {:?} -> {:?}!",
                act, syn
            )))),

            /*** Errors. ***/
            (_, Err(ErrType::Undefined)) => {
                Some(Err(DataError::TypeError(String::from("Undefined type!"))))
            }
            (_, Err(ErrType::NotImplemented(typ))) => Some(Err(
                DataError::TypeError(format!("Unimplemented type: {:?}", typ)),
            )),
            (_, Err(_)) => None,
        }
    }

    pub fn fixed_index_length(&self) -> Option<usize> {
        match self.syntax {
            VarType::Boolean
            | VarType::Integer
            | VarType::Integer32
            | VarType::Integer64
            | VarType::Unsigned64
            | VarType::Counter
            | VarType::Counter64
            | VarType::Gauge
            | VarType::TimeTicks => Some(1),
            VarType::IpAddress => Some(4),
            VarType::MacAddress => Some(6),
            VarType::Oid | VarType::OctetStr => {
                match self.value_range.as_ref().map(ValueRange::normalize) {
                    Some(ValueRange::Exact(n)) => Some(n as usize),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    pub fn variable_index_length<'a>(
        &self,
        index: &'a [u64],
        implied_length: Option<usize>,
    ) -> Result<(usize, &'a [u64]), DataError> {
        match implied_length {
            Some(len) => Ok((len, index)),
            None => {
                match self.value_range.as_ref().map(ValueRange::normalize) {
                    Some(ValueRange::Exact(n))
                        if n >= 0 && n as usize <= index.len() =>
                    {
                        Ok((n as usize, index))
                    }
                    _ => match index.split_first() {
                        Some((n, next)) if *n as usize <= index.len() => {
                            Ok((*n as usize, next))
                        }
                        Some(_) => Err(DataError::TypeError(String::from(
                            "variable index length value is invalid",
                        ))),
                        None => Err(DataError::TypeError(String::from(
                            "missing length for string index",
                        ))),
                    },
                }
            }
        }
    }

    pub fn get_value_from_index<'a>(
        &self,
        index: &'a [u64],
        implied_length: Option<usize>,
    ) -> Result<(Data, &'a [u64]), DataError> {
        match self.syntax {
            VarType::Integer => {
                let (v, next) = self.get_integer_index_value(index)?;
                Ok((self.integer_enum_from_value(v), next))
            }
            VarType::Integer32
            | VarType::Integer64
            | VarType::Unsigned64
            | VarType::Gauge
            | VarType::Counter
            | VarType::Counter64 => {
                let (v, next) = self.get_integer_index_value(index)?;
                Ok((Ok(Value::Integer(v)), next))
            }
            VarType::IpAddress => match index.len() >= 4 {
                true => Ok((
                    match index[..4].iter().all(|v| *v < 256) {
                        true => Ok(Value::Ipv4Address([
                            index[0] as u8,
                            index[1] as u8,
                            index[2] as u8,
                            index[3] as u8,
                        ])),
                        false => Err(DataError::TypeError(String::from(
                            "invalid index value for IpAddress",
                        ))),
                    },
                    &index[4..],
                )),
                false => Err(DataError::TypeError(String::from(
                    "index too short for IpAddress",
                ))),
            },
            VarType::MacAddress => match index.len() >= 6 {
                true => Ok((
                    match index[..6].iter().all(|v| *v < 256) {
                        true => Ok(Value::MacAddress([
                            index[0] as u8,
                            index[1] as u8,
                            index[2] as u8,
                            index[3] as u8,
                            index[4] as u8,
                            index[5] as u8,
                        ])),
                        false => Err(DataError::TypeError(String::from(
                            "invalid index value for MacAddress",
                        ))),
                    },
                    &index[6..],
                )),
                false => Err(DataError::TypeError(String::from(
                    "index too short for MacAddress",
                ))),
            },

            VarType::Oid => {
                let (len, index) =
                    self.variable_index_length(index, implied_length)?;
                let (data, next) = index.split_at(len);
                Ok((
                    Ok(Value::UnicodeString(Oid::from_slice(data).to_string())),
                    next,
                ))
            }

            VarType::OctetStr => {
                let (len, index) =
                    self.variable_index_length(index, implied_length)?;
                let (data, next) = index.split_at(len);
                let data = data.iter().map(|i| *i as u8).collect();
                Ok((self.string_enum_from_value(data), next))
            }

            _ => Err(DataError::TypeError(String::from(
                "unsupported type for index",
            ))),
        }
    }

    fn get_integer_index_value<'a>(
        &self,
        index: &'a [u64],
    ) -> Result<(i64, &'a [u64]), DataError> {
        match index.split_first() {
            Some((v, next)) => Ok((*v as i64, next)),
            None => {
                Err(DataError::TypeError(String::from("missing index value")))
            }
        }
    }

    fn integer_enum_from_value(&self, v: i64) -> Data {
        match &self.value_list {
            Some(ValueList::Integer(cs)) => match self.error_enum {
                false => Ok(Value::IntEnum(IntEnumValue::new(cs.clone(), v)?)),
                true => Ok(Value::Result(ResultValue::new(
                    Arc::new(Type::Integer),
                    Arc::new(Type::IntEnum(cs.clone())),
                    match cs.contains_key(&v) {
                        false => Ok(Value::Integer(v)),
                        true => Err(Value::IntEnum(IntEnumValue::new(
                            cs.clone(),
                            v,
                        )?)),
                    },
                )?)),
            },
            Some(_) => Err(DataError::TypeError(
                "expected integer ValueMap".to_string(),
            )),
            None => Ok(Value::Integer(v)),
        }
    }

    fn bitstr_set_from_value(&self, v: Vec<u8>) -> Data {
        match &self.value_list {
            Some(ValueList::Integer(cs)) => Ok(Value::Set(SetValue::new(
                Arc::new(HashableType::IntEnum(cs.clone())),
                cs.iter()
                    .filter_map(|(i, _)| {
                        match v
                            .get((i / 8) as usize)
                            .map_or(false, |b| b & (1 << (i % 8)) > 0)
                        {
                            true => Some(
                                IntEnumValue::new(cs.clone(), *i)
                                    .map(HashableValue::IntEnum),
                            ),
                            false => None,
                        }
                    })
                    .collect::<Result<_, DataError>>()?,
            )?)),
            _ => Err(DataError::TypeError(
                "expected integer ValueMap for BitStr".to_string(),
            )),
        }
    }

    fn string_enum_from_value(&self, v: Vec<u8>) -> Data {
        match &self.value_list {
            Some(ValueList::String(cs)) => {
                let s = String::from_utf8_lossy(&v).to_string();
                Ok(Value::Enum(EnumValue::new(
                    Arc::new(cs.values().cloned().collect()),
                    s,
                )?))
            }
            Some(_) => Err(DataError::TypeError(
                "expected string ValueMap".to_string(),
            )),
            None => Ok(Value::BinaryString(v)),
        }
    }
}

impl ValueRange {
    fn normalize(&self) -> Self {
        match self {
            Self::Exact(n) => Self::Exact(*n),
            Self::Range {
                from: Some(from),
                to: Some(to),
            } if from == to => Self::Exact(*from),
            Self::Range {
                from: Some(from),
                to: Some(to),
            } if from > to => Self::Range {
                from: Some(*to),
                to: Some(*from),
            },
            _ => self.clone(),
        }
    }
}
