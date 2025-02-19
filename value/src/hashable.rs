/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{BTreeMap, BTreeSet};
use std::convert::TryInto;
use std::fmt::Display;
use std::str::FromStr;
use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[cfg(feature = "dbschema")]
use dbschema::{
    BoolSchema, DbSchema, EnumSchema, IntegerSchema, Ipv4Schema, Ipv6Schema,
    ListSchema, OptionSchema, StringSchema, StructSchema, UnitSchema,
};

use crate::value::{
    EnumValue, IntEnumValue, ListValue, OptionValue, ResultValue,
};
use crate::{DataError, TypeOpts};

use super::types::Type;
use super::value::Value;

#[derive(
    Serialize, Deserialize, Clone, Hash, Debug, PartialEq, Eq, PartialOrd, Ord,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum HashableType {
    BinaryString,
    UnicodeString,
    Integer,
    #[serde(rename = "set-enum")]
    Enum(Arc<BTreeSet<String>>),
    #[serde(rename = "int-enum")]
    #[serde(alias = "enum")]
    IntEnum(
        #[serde(with = "agent_serde::arc_intkey_map")]
        #[cfg_attr(
            feature = "schemars",
            schemars(with = "BTreeMap<String, String>")
        )]
        Arc<BTreeMap<i64, String>>,
    ),
    Boolean,
    MacAddress,
    #[serde(rename = "ipv4addr")]
    #[serde(alias = "ipaddr")]
    Ipv4Address,
    #[serde(rename = "ipv6addr")]
    Ipv6Address,
    Option(Arc<HashableType>),
    Result(Arc<HashableType>, Arc<HashableType>),
    List(Arc<HashableType>),
    Tuple(Vec<HashableType>),
}

#[derive(
    Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum HashableValue {
    BinaryString(Vec<u8>),
    UnicodeString(String),
    Integer(i64),
    Enum(EnumValue),
    IntEnum(IntEnumValue),
    Boolean(bool),
    MacAddress([u8; 6]),
    Ipv4Address([u8; 4]),
    Ipv6Address([u16; 8]),
    Option(HashableOptionValue),
    Result(HashableResultValue),
    List(HashableListValue),
    Tuple(Vec<HashableValue>),
}

#[derive(
    Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct HashableOptionValue(Arc<HashableType>, Option<Box<HashableValue>>);

#[derive(
    Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct HashableResultValue(
    Arc<HashableType>,
    Arc<HashableType>,
    Result<Box<HashableValue>, Box<HashableValue>>,
);

#[derive(
    Serialize, Deserialize, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct HashableListValue(Arc<HashableType>, Vec<HashableValue>);

impl HashableType {
    pub fn get_type(&self) -> Type {
        match self {
            HashableType::BinaryString => Type::BinaryString,
            HashableType::UnicodeString => Type::UnicodeString,
            HashableType::Integer => Type::Integer,
            HashableType::Enum(cs) => Type::Enum(cs.clone()),
            HashableType::IntEnum(cs) => Type::IntEnum(cs.clone()),
            HashableType::Boolean => Type::Boolean,
            HashableType::MacAddress => Type::MacAddress,
            HashableType::Ipv4Address => Type::Ipv4Address,
            HashableType::Ipv6Address => Type::Ipv6Address,
            HashableType::Option(t) => Type::Option(Arc::new(t.get_type())),
            HashableType::Result(t, e) => {
                Type::Result(Arc::new(t.get_type()), Arc::new(e.get_type()))
            }
            HashableType::List(t) => Type::List(Arc::new(t.get_type())),
            HashableType::Tuple(ts) => {
                Type::Tuple(ts.iter().map(|t| t.get_type()).collect())
            }
        }
    }

    pub fn castable_to_opts(
        &self,
        target: &HashableType,
        opts: &TypeOpts,
    ) -> bool {
        target == self
            || match (target, self) {
                (Self::BinaryString, Self::UnicodeString)
                | (Self::UnicodeString, Self::BinaryString) => {
                    !opts.strict_strings
                }
                (Self::Option(s), Self::Option(t)) => {
                    t.castable_to_opts(s, opts)
                }
                (Self::Result(s, d), Self::Result(t, e)) => {
                    t.castable_to_opts(s, opts) && e.castable_to_opts(d, opts)
                }
                (Self::Tuple(ss), Self::Tuple(ts)) => {
                    ss.len() == ts.len()
                        && ss
                            .iter()
                            .zip(ts)
                            .all(|(s, t)| t.castable_to_opts(s, opts))
                }
                (Self::List(s), Self::List(t)) => t.castable_to_opts(s, opts),
                _ => false,
            }
    }

    pub fn key_from_json(
        &self,
        value: String,
    ) -> std::result::Result<HashableValue, DataError> {
        match self {
            HashableType::UnicodeString => {
                Ok(HashableValue::UnicodeString(value))
            }
            _ => {
                let value = serde_json::from_str(&value)
                    .map_err(|e| DataError::Json(e.to_string()))?;
                self.value_from_json(value)
            }
        }
    }

    // TODO: agree with dbschema function!
    pub fn value_from_json(
        &self,
        value: serde_json::Value,
    ) -> std::result::Result<HashableValue, DataError> {
        fn decode<T: DeserializeOwned>(
            value: serde_json::Value,
        ) -> std::result::Result<T, DataError> {
            serde_json::from_value(value)
                .map_err(|e| DataError::Json(e.to_string()))
        }

        match self {
            HashableType::BinaryString => {
                Ok(HashableValue::BinaryString(decode(value)?))
            }
            HashableType::UnicodeString => {
                Ok(HashableValue::UnicodeString(decode(value)?))
            }
            HashableType::Integer => Ok(HashableValue::Integer(decode(value)?)),
            HashableType::Boolean => Ok(HashableValue::Boolean(decode(value)?)),
            HashableType::Enum(cs) => Ok(HashableValue::Enum(EnumValue::new(
                cs.clone(),
                decode(value)?,
            )?)),
            HashableType::IntEnum(cs) => Ok(HashableValue::IntEnum(
                IntEnumValue::new(cs.clone(), decode(value)?)?,
            )),
            HashableType::MacAddress => {
                let s: String = decode(value)?;
                Ok(HashableValue::MacAddress(
                    s.split(':')
                        .map(|n| u8::from_str_radix(n, 16))
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| {
                            DataError::InvalidMacAddress(s.to_string())
                        })?
                        .try_into()
                        .map_err(|_| {
                            DataError::InvalidMacAddress(s.to_string())
                        })?,
                ))
            }
            HashableType::Ipv4Address => {
                let s: String = decode(value)?;
                Ok(HashableValue::Ipv4Address(
                    std::net::Ipv4Addr::from_str(&s)
                        .map_err(|_| DataError::InvalidIpv4Address(s))?
                        .octets(),
                ))
            }
            HashableType::Ipv6Address => {
                let s: String = decode(value)?;
                Ok(HashableValue::Ipv6Address(
                    std::net::Ipv6Addr::from_str(&s)
                        .map_err(|_| {
                            DataError::InvalidIpv6Address(s.to_string())
                        })?
                        .segments(),
                ))
            }
            HashableType::Option(t) => {
                Ok(HashableValue::Option(HashableOptionValue(
                    t.clone(),
                    match value {
                        serde_json::Value::Null => None,
                        _ => Some(Box::new(t.value_from_json(value)?)),
                    },
                )))
            }
            HashableType::Result(t, e) => {
                Ok(HashableValue::Result(HashableResultValue(
                    t.clone(),
                    e.clone(),
                    match decode(value)? {
                        Ok(v) => Ok(Box::new(t.value_from_json(v)?)),
                        Err(v) => Ok(Box::new(e.value_from_json(v)?)),
                    },
                )))
            }
            HashableType::Tuple(ts) => Ok(HashableValue::Tuple(
                ts.iter()
                    .zip(decode::<Vec<serde_json::Value>>(value)?)
                    .map(|(t, v)| t.value_from_json(v))
                    .collect::<Result<_, _>>()?,
            )),
            HashableType::List(t) => {
                Ok(HashableValue::List(HashableListValue(
                    t.clone(),
                    decode::<Vec<serde_json::Value>>(value)?
                        .into_iter()
                        .map(|v| t.value_from_json(v))
                        .collect::<Result<_, _>>()?,
                )))
            }
        }
    }

    #[cfg(feature = "dbschema")]
    pub fn dbschema(&self) -> DbSchema {
        match self {
            HashableType::BinaryString => {
                ListSchema::new(IntegerSchema::new()).into()
            }
            HashableType::UnicodeString => StringSchema::new().into(),
            HashableType::Integer => IntegerSchema::new().into(),
            HashableType::Enum(cs) => cs
                .iter()
                .fold(EnumSchema::new(), |schema, choice| {
                    schema.option(choice, UnitSchema::new())
                })
                .tag_string()
                .into(),
            HashableType::IntEnum(cs) => cs
                .values()
                .fold(EnumSchema::new(), |schema, choice| {
                    schema.option(choice, UnitSchema::new())
                })
                .tag_string()
                .into(),
            HashableType::Boolean => BoolSchema::new().into(),
            HashableType::MacAddress => StringSchema::new().into(),
            HashableType::Ipv4Address => Ipv4Schema::new().into(),
            HashableType::Ipv6Address => Ipv6Schema::new().into(),
            HashableType::Option(t) => OptionSchema::new(t.dbschema()).into(),
            HashableType::Result(t, e) => EnumSchema::new()
                .option("ok", t.dbschema())
                .option("err", e.dbschema())
                .into(),
            HashableType::List(t) => ListSchema::new(t.dbschema()).into(),
            HashableType::Tuple(ts) => ts
                .iter()
                .enumerate()
                .fold(StructSchema::new(), |schema, (i, t)| {
                    schema.field(i.to_string(), t.dbschema())
                })
                .into(),
        }
    }
}

impl HashableValue {
    pub fn get_type(&self) -> HashableType {
        match self {
            HashableValue::BinaryString(_) => HashableType::BinaryString,
            HashableValue::UnicodeString(_) => HashableType::UnicodeString,
            HashableValue::Enum(v) => HashableType::Enum(v.choices().clone()),
            HashableValue::IntEnum(v) => {
                HashableType::IntEnum(v.choices().clone())
            }
            HashableValue::Integer(_) => HashableType::Integer,
            HashableValue::Boolean(_) => HashableType::Boolean,
            HashableValue::MacAddress(_) => HashableType::MacAddress,
            HashableValue::Ipv4Address(_) => HashableType::Ipv4Address,
            HashableValue::Ipv6Address(_) => HashableType::Ipv6Address,
            HashableValue::Option(v) => HashableType::Option(v.0.clone()),
            HashableValue::Result(v) => {
                HashableType::Result(v.0.clone(), v.1.clone())
            }
            HashableValue::Tuple(vs) => {
                HashableType::Tuple(vs.iter().map(|t| t.get_type()).collect())
            }
            HashableValue::List(v) => HashableType::List(v.0.clone()),
        }
    }

    // Should agree with HashableType::castable_to!
    pub fn cast_to_opts(
        self,
        target: &HashableType,
        opts: &TypeOpts,
    ) -> Result<HashableValue, DataError> {
        let source = self.get_type();
        match target == &source {
            true => Ok(self),
            false => match (target, self) {
                (HashableType::BinaryString, Self::UnicodeString(s)) => {
                    match &opts.strict_strings {
                        false => Ok(HashableValue::BinaryString(
                            s.as_bytes().to_vec(),
                        )),
                        true => Err(DataError::TypeError(
                            "implicit casts between binary and \
							 unicode strings are disabled"
                                .to_string(),
                        )),
                    }
                }
                (HashableType::UnicodeString, Self::BinaryString(bs)) => {
                    match &opts.strict_strings {
                        false => Ok(HashableValue::UnicodeString(
                            String::from_utf8_lossy(bs.as_slice()).to_string(),
                        )),
                        true => Err(DataError::TypeError(
                            "implicit casts between binary and \
							 unicode strings are disabled"
                                .to_string(),
                        )),
                    }
                }
                (
                    HashableType::Option(t),
                    Self::Option(HashableOptionValue(_, v)),
                ) => match v {
                    Some(v) => Ok(Self::Option(HashableOptionValue(
                        t.clone(),
                        Some(Box::new(v.clone().cast_to_opts(t, opts)?)),
                    ))),
                    None => {
                        Ok(Self::Option(HashableOptionValue(t.clone(), None)))
                    }
                },
                (
                    HashableType::Result(t, e),
                    Self::Result(HashableResultValue(_, _, v)),
                ) => match v {
                    Ok(v) => Ok(Self::Result(HashableResultValue(
                        t.clone(),
                        e.clone(),
                        Ok(Box::new(v.clone().cast_to_opts(t, opts)?)),
                    ))),
                    Err(v) => Ok(Self::Result(HashableResultValue(
                        t.clone(),
                        e.clone(),
                        Err(Box::new(v.clone().cast_to_opts(e, opts)?)),
                    ))),
                },
                (HashableType::Tuple(ts), Self::Tuple(vs))
                    if ts.len() == vs.len() =>
                {
                    Ok(Self::Tuple(
                        ts.iter()
                            .zip(vs)
                            .map(|(t, v)| v.cast_to_opts(t, opts))
                            .collect::<Result<_, DataError>>()?,
                    ))
                }
                (
                    HashableType::List(t),
                    Self::List(HashableListValue(_, vs)),
                ) => Ok(Self::List(HashableListValue(
                    t.clone(),
                    vs.into_iter()
                        .map(|v| v.cast_to_opts(t, opts))
                        .collect::<Result<_, DataError>>()?,
                ))),

                _ => Err(DataError::TypeError(format!(
                    "expected {}, got {}",
                    target, source
                ))),
            },
        }
    }

    pub fn from_value(val: Value) -> Option<Self> {
        match val {
            Value::BinaryString(v) => Some(HashableValue::BinaryString(v)),
            Value::UnicodeString(v) => Some(HashableValue::UnicodeString(v)),
            Value::Enum(v) => Some(HashableValue::Enum(v)),
            Value::IntEnum(v) => Some(HashableValue::IntEnum(v)),
            Value::Integer(v) => Some(HashableValue::Integer(v)),
            Value::Boolean(v) => Some(HashableValue::Boolean(v)),
            Value::MacAddress(v) => Some(HashableValue::MacAddress(v)),
            Value::Ipv4Address(v) => Some(HashableValue::Ipv4Address(v)),
            Value::Ipv6Address(v) => Some(HashableValue::Ipv6Address(v)),
            Value::Option(v) => {
                Some(HashableValue::Option(HashableOptionValue::from_value(v)?))
            }
            Value::Result(v) => {
                Some(HashableValue::Result(HashableResultValue::from_value(v)?))
            }
            Value::Tuple(vs) => match vs
                .into_iter()
                .map(|v| HashableValue::from_value(v).ok_or(()))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(hvs) => Some(HashableValue::Tuple(hvs)),
                Err(()) => None,
            },
            _ => None,
        }
    }

    pub fn to_value(self) -> Value {
        match self {
            HashableValue::BinaryString(v) => Value::BinaryString(v),
            HashableValue::UnicodeString(v) => Value::UnicodeString(v),
            HashableValue::Integer(v) => Value::Integer(v),
            HashableValue::Enum(v) => Value::Enum(v),
            HashableValue::IntEnum(v) => Value::IntEnum(v),
            HashableValue::Boolean(v) => Value::Boolean(v),
            HashableValue::MacAddress(v) => Value::MacAddress(v),
            HashableValue::Ipv4Address(v) => Value::Ipv4Address(v),
            HashableValue::Ipv6Address(v) => Value::Ipv6Address(v),
            HashableValue::Option(v) => Value::Option(v.to_value()),
            HashableValue::Result(v) => Value::Result(v.to_value()),
            HashableValue::List(v) => Value::List(v.to_value()),
            HashableValue::Tuple(vs) => {
                Value::Tuple(vs.into_iter().map(|v| v.to_value()).collect())
            }
        }
    }

    pub fn to_json_key(&self) -> std::result::Result<String, String> {
        match self {
            Self::UnicodeString(s) => Ok(s.to_string()),
            _ => serde_json::to_string(&self.to_json_value()?)
                .map_err(|e| e.to_string()),
        }
    }

    // TODO: agree with dbschema!
    pub fn to_json_value(
        &self,
    ) -> std::result::Result<serde_json::Value, String> {
        match self {
            HashableValue::BinaryString(v) => Ok(json!(v)),
            HashableValue::UnicodeString(v) => Ok(json!(v)),
            HashableValue::Integer(v) => Ok(json!(v)),
            HashableValue::Enum(v) => Ok(json!(v.get_value())),
            HashableValue::IntEnum(v) => Ok(json!(v.get_value_str())),
            HashableValue::Boolean(v) => Ok(json!(v)),
            HashableValue::MacAddress(v) => Ok(json!(format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                v[0], v[1], v[2], v[3], v[4], v[5]
            ))),
            HashableValue::Ipv4Address(v) => {
                Ok(json!(format!("{}.{}.{}.{}", v[0], v[1], v[2], v[3])))
            }
            HashableValue::Ipv6Address(v) => Ok(json!(format!(
                "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
            ))),
            HashableValue::Option(v) => match &v.1 {
                Some(v) => v.to_json_value(),
                None => Ok(json!(null)),
            },
            HashableValue::Result(v) => match &v.2 {
                Ok(v) => Ok(json!({"ok": v.to_json_value()?})),
                Err(v) => Ok(json!({"err": v.to_json_value()?})),
            },
            HashableValue::List(v) => Ok(serde_json::Value::Array(
                v.1.iter()
                    .map(|v| v.to_json_value())
                    .collect::<Result<_, String>>()?,
            )),
            HashableValue::Tuple(v) => Ok(serde_json::Value::Array(
                v.iter()
                    .map(|v| v.to_json_value())
                    .collect::<Result<_, String>>()?,
            )),
        }
    }
}

impl HashableOptionValue {
    pub fn new(
        typ: Arc<HashableType>,
        value: Option<HashableValue>,
    ) -> Result<Self, DataError> {
        match value {
            Some(value) => match &value.get_type() == typ.as_ref() {
                true => Ok(Self(typ, Some(Box::new(value)))),
                false => Err(DataError::InvalidOptionValue),
            },
            None => Ok(Self(typ, None)),
        }
    }

    pub fn get_type(&self) -> HashableType {
        HashableType::Option(self.0.clone())
    }

    pub fn get_value(&self) -> Option<&HashableValue> {
        self.1.as_deref()
    }

    pub fn from_value(value: OptionValue) -> Option<Self> {
        let (typ, value) = value.deconstruct();
        let typ = Arc::new(typ.hashable()?);
        match value {
            Some(value) => Some(Self(
                typ,
                Some(Box::new(HashableValue::from_value(value)?)),
            )),
            None => Some(Self(typ, None)),
        }
    }

    pub fn to_value(self) -> OptionValue {
        OptionValue::new_unchecked(
            Arc::new(self.0.get_type()),
            self.1.map(|v| v.to_value()),
        )
    }
}

impl HashableResultValue {
    pub fn new(
        ok: Arc<HashableType>,
        err: Arc<HashableType>,
        value: Result<HashableValue, HashableValue>,
    ) -> Result<Self, DataError> {
        match value {
            Ok(value) => match &value.get_type() == ok.as_ref() {
                true => Ok(Self(ok, err, Ok(Box::new(value)))),
                false => Err(DataError::InvalidResultValue),
            },
            Err(value) => match &value.get_type() == err.as_ref() {
                true => Ok(Self(ok, err, Err(Box::new(value)))),
                false => Err(DataError::InvalidResultValue),
            },
        }
    }

    pub fn get_type(&self) -> HashableType {
        HashableType::Result(self.0.clone(), self.1.clone())
    }

    pub fn get_value(&self) -> Result<&HashableValue, &HashableValue> {
        match &self.2 {
            Ok(v) => Ok(v.as_ref()),
            Err(v) => Err(v.as_ref()),
        }
    }

    pub fn from_value(value: ResultValue) -> Option<Self> {
        let (ok, err, value) = value.deconstruct();
        let ok = Arc::new(ok.hashable()?);
        let err = Arc::new(err.hashable()?);
        match value {
            Ok(value) => Some(Self(
                ok,
                err,
                Ok(Box::new(HashableValue::from_value(value)?)),
            )),
            Err(value) => Some(Self(
                ok,
                err,
                Err(Box::new(HashableValue::from_value(value)?)),
            )),
        }
    }

    pub fn to_value(self) -> ResultValue {
        ResultValue::new_unchecked(
            Arc::new(self.0.get_type()),
            Arc::new(self.1.get_type()),
            self.2.map(|v| v.to_value()).map_err(|v| v.to_value()),
        )
    }
}

impl HashableListValue {
    pub fn get_values(&self) -> &[HashableValue] {
        &self.1
    }

    pub fn to_value(self) -> ListValue {
        ListValue::new_unchecked(
            Arc::new(self.0.get_type()),
            self.1.into_iter().map(|v| v.to_value()).collect(),
        )
    }
}

impl Display for HashableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashableType::BinaryString => write!(f, "binarystring"),
            HashableType::UnicodeString => write!(f, "unicodestring"),
            HashableType::Integer => write!(f, "integer"),
            HashableType::Enum(cs) => write!(f, "enum({:?})", cs),
            HashableType::IntEnum(cs) => write!(f, "int_enum({:?})", cs),
            HashableType::Boolean => write!(f, "boolean"),
            HashableType::MacAddress => write!(f, "macaddr"),
            HashableType::Ipv4Address => write!(f, "ipaddr"),
            HashableType::Ipv6Address => write!(f, "ipv6addr"),
            HashableType::Option(t) => write!(f, "option({t})"),
            HashableType::Result(t, e) => write!(f, "result({t},{e})"),
            HashableType::List(t) => write!(f, "list({t})"),
            HashableType::Tuple(ts) => write!(
                f,
                "tuple({})",
                ts.iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

impl Display for HashableValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HashableValue::BinaryString(v) => {
                write!(f, "{:?}", String::from_utf8_lossy(v))
            }
            HashableValue::UnicodeString(v) => write!(f, "{v:?}"),
            HashableValue::Integer(v) => write!(f, "{v}"),
            HashableValue::Enum(v) => write!(f, "{}", v.get_value()),
            HashableValue::IntEnum(v) => {
                write!(f, "{}", v.get_value_str())
            }
            HashableValue::Boolean(v) => write!(f, "{v}"),
            HashableValue::MacAddress(v) => write!(
                f,
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                v[0], v[1], v[2], v[3], v[4], v[5]
            ),
            HashableValue::Ipv4Address(v) => {
                write!(f, "{},{}.{}.{}", v[0], v[1], v[2], v[3])
            }
            HashableValue::Ipv6Address(v) => write!(
                f,
                "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
            ),
            HashableValue::Option(v) => match v.get_value() {
                None => write!(f, "None"),
                Some(v) => write!(f, "{v}"),
            },
            HashableValue::Result(v) => match v.get_value() {
                Ok(v) => write!(f, "Ok({v})"),
                Err(e) => write!(f, "Err({e})"),
            },
            HashableValue::List(v) => {
                write!(f, "[")?;
                let mut vs = v.get_values().iter();
                vs.next().into_iter().try_for_each(|v| write!(f, "{v}"))?;
                vs.try_for_each(|v| write!(f, ", {v}"))?;
                write!(f, "]")
            }
            HashableValue::Tuple(vs) => {
                write!(f, "(")?;
                let mut vs = vs.iter();
                vs.next().into_iter().try_for_each(|v| write!(f, "{v}"))?;
                vs.try_for_each(|v| write!(f, ", {v}"))?;
                write!(f, ")")
            }
        }
    }
}
