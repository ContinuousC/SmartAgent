/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::convert::TryInto;
use std::fmt::{self, Display};
use std::str::FromStr;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use unit::{Dimension, Quantity, Unit};

#[cfg(feature = "dbschema")]
use dbschema::{
    BoolSchema, DateTimeSchema, DbSchema, DictionarySchema, DoubleSchema,
    EnumSchema, IntegerSchema, Ipv4Schema, Ipv6Schema, JsonSchema, ListSchema,
    OptionSchema, SetSchema, StringSchema, StructSchema, UnitSchema,
};

use crate::hashable::HashableType;
use crate::value::{
    EnumValue, IntEnumValue, ListValue, MapValue, OptionValue, ResultValue,
    SetValue,
};

use super::error::{Data, DataError};
use super::options::TypeOpts;
use super::value::Value;

/// Possible types a value can take.
#[derive(
    Serialize, Deserialize, Clone, Hash, Debug, PartialEq, PartialOrd, Eq, Ord,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Type {
    BinaryString,
    #[serde(alias = "string")]
    UnicodeString,
    Integer,
    Float,
    Quantity(Dimension),
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
    Time,
    Age,
    #[serde(rename = "macaddr")]
    MacAddress,
    #[serde(rename = "ipv4addr")]
    #[serde(alias = "ipaddr")]
    Ipv4Address,
    #[serde(rename = "ipv6addr")]
    Ipv6Address,
    Option(Arc<Type>),
    Result(Arc<Type>, Arc<Type>),
    Tuple(Vec<Type>),
    List(Arc<Type>),
    Set(Arc<HashableType>),
    Map(Arc<HashableType>, Arc<Type>),
    Json,
}

impl Type {
    pub fn is_hashable(&self) -> bool {
        match self {
            Type::BinaryString => true,
            Type::UnicodeString => true,
            Type::Integer => true,
            Type::Enum(_) => true,
            Type::IntEnum(_) => true,
            Type::Boolean => true,
            Type::MacAddress => true,
            Type::Ipv4Address => true,
            Type::Ipv6Address => true,
            Type::Option(t) => t.is_hashable(),
            Type::Result(t, e) => t.is_hashable() && e.is_hashable(),
            Type::Tuple(ts) => ts.iter().all(|t| t.is_hashable()),
            Type::List(t) => t.is_hashable(),
            Type::Set(_) => false, /* Currently HashSet is not itself hashable. */
            Type::Map(_, _) => false, /* Currently HashMap is not itself hashable. */
            Type::Float => false,
            Type::Quantity(_) => false,
            Type::Time => false,
            Type::Age => false,
            Type::Json => false,
        }
    }

    pub fn hashable(&self) -> Option<HashableType> {
        match self {
            Type::BinaryString => Some(HashableType::BinaryString),
            Type::UnicodeString => Some(HashableType::UnicodeString),
            Type::Integer => Some(HashableType::Integer),
            Type::Enum(cs) => Some(HashableType::Enum(cs.clone())),
            Type::IntEnum(cs) => Some(HashableType::IntEnum(cs.clone())),
            Type::Boolean => Some(HashableType::Boolean),
            Type::MacAddress => Some(HashableType::MacAddress),
            Type::Ipv4Address => Some(HashableType::Ipv4Address),
            Type::Ipv6Address => Some(HashableType::Ipv6Address),
            Type::Option(t) => {
                Some(HashableType::Option(Arc::new(t.hashable()?)))
            }
            Type::Result(t, e) => Some(HashableType::Result(
                Arc::new(t.hashable()?),
                Arc::new(e.hashable()?),
            )),
            Type::Tuple(ts) => Some(HashableType::Tuple(
                ts.iter()
                    .map(|t| t.hashable())
                    .collect::<Option<Vec<_>>>()?,
            )),
            Type::List(t) => Some(HashableType::List(Arc::new(t.hashable()?))),
            Type::Float
            | Type::Quantity(_)
            | Type::Time
            | Type::Age
            | Type::Set(_)
            | Type::Map(_, _)
            | Type::Json => None,
        }
    }

    pub fn castable_to(&self, target: &Type) -> bool {
        self.castable_to_opts(target, &TypeOpts::default())
    }

    // Should agree with Value::cast_to!
    pub fn castable_to_opts(&self, target: &Type, opts: &TypeOpts) -> bool {
        target == self
            || match (target, self) {
                (Type::BinaryString, Type::UnicodeString)
                | (Type::UnicodeString, Type::BinaryString) => {
                    !opts.strict_strings
                }
                (
                    Type::Quantity(Dimension::Dimensionless),
                    Type::Integer | Type::Float,
                ) => true,
                (Type::Float, Type::Integer) => true,
                (Type::Option(s), Type::Option(t)) => {
                    t.castable_to_opts(s, opts)
                }
                (Type::Result(s, d), Type::Result(t, e)) => {
                    t.castable_to(s) && e.castable_to_opts(d, opts)
                }
                (Type::List(s), Type::List(t)) => t.castable_to_opts(s, opts),
                (Type::Set(s), Type::Set(t)) => t.castable_to_opts(s, opts),
                (Type::Map(j, u), Type::Map(k, v)) => {
                    k.castable_to_opts(j, opts) && v.castable_to_opts(u, opts)
                }
                (Type::Tuple(ss), Type::Tuple(ts)) => {
                    ss.len() == ts.len()
                        && ss
                            .iter()
                            .zip(ts)
                            .all(|(s, t)| t.castable_to_opts(s, opts))
                }
                _ => false,
            }
    }

    pub fn value_from_json(&self, value: serde_json::Value) -> Data {
        self.value_from_json_unit(value, None)
    }

    pub fn value_from_json_unit(
        &self,
        value: serde_json::Value,
        display_unit: Option<Unit>,
    ) -> Data {
        fn decode<T: DeserializeOwned>(
            value: serde_json::Value,
        ) -> std::result::Result<T, DataError> {
            serde_json::from_value(value)
                .map_err(|e| DataError::Json(e.to_string()))
        }

        match self {
            Type::UnicodeString => Ok(Value::UnicodeString(decode(value)?)),
            Type::BinaryString => Ok(Value::BinaryString(decode(value)?)),
            Type::Integer => Ok(Value::Integer(decode(value)?)),
            Type::Float => Ok(Value::Float(decode(value)?)),
            Type::Quantity(dim) => Ok(Value::Quantity(Quantity(
                decode(value)?,
                display_unit
                    .map_or_else(|| dim.reference_unit(), |u| u.normalize()),
            ))),
            Type::Enum(cs) => {
                Ok(Value::Enum(EnumValue::new(cs.clone(), decode(value)?)?))
            }
            Type::IntEnum(cs) => Ok(Value::IntEnum(IntEnumValue::new(
                cs.clone(),
                decode(value)?,
            )?)),
            Type::Boolean => Ok(Value::Boolean(decode(value)?)),
            Type::Time => {
                let s: String = decode(value)?;
                Ok(Value::Time(
                    DateTime::parse_from_rfc3339(s.as_str())
                        .map_err(|e| DataError::Json(e.to_string()))?
                        .with_timezone(&Utc),
                ))
            }
            Type::Age => {
                let seconds: f64 = decode(value)?;
                Ok(Value::Age(Duration::milliseconds(f64::round(
                    seconds * 1000.0,
                ) as i64)))
            }
            Type::MacAddress => {
                let s: String = decode(value)?;
                Ok(Value::MacAddress(
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
            Type::Ipv4Address => {
                let s: String = decode(value)?;
                Ok(Value::Ipv4Address(
                    std::net::Ipv4Addr::from_str(&s)
                        .map_err(|_| DataError::InvalidIpv4Address(s))?
                        .octets(),
                ))
            }
            Type::Ipv6Address => {
                let s: String = decode(value)?;
                Ok(Value::Ipv6Address(
                    std::net::Ipv6Addr::from_str(&s)
                        .map_err(|_| {
                            DataError::InvalidIpv6Address(s.to_string())
                        })?
                        .segments(),
                ))
            }
            Type::Option(typ) => Ok(Value::Option(OptionValue::new_unchecked(
                typ.clone(),
                match value {
                    serde_json::Value::Null => None,
                    _ => Some(typ.value_from_json_unit(value, display_unit)?),
                },
            ))),
            Type::Result(ok, err) => {
                Ok(Value::Result(ResultValue::new_unchecked(
                    ok.clone(),
                    err.clone(),
                    match decode(value)? {
                        Ok(v) => Ok(ok.value_from_json(v)?),
                        Err(e) => Err(err.value_from_json(e)?),
                    },
                )))
            }
            Type::Tuple(ts) => Ok(Value::Tuple(
                ts.iter()
                    .zip(decode::<Vec<serde_json::Value>>(value)?)
                    .map(|(t, v)| t.value_from_json_unit(v, display_unit))
                    .collect::<Result<_, _>>()?,
            )),

            Type::List(typ) => Ok(Value::List(ListValue::new_unchecked(
                typ.clone(),
                decode::<Vec<serde_json::Value>>(value)?
                    .into_iter()
                    .map(|v| typ.value_from_json_unit(v, display_unit))
                    .collect::<Result<_, _>>()?,
            ))),
            Type::Set(typ) => Ok(Value::Set(SetValue::new_unchecked(
                typ.clone(),
                decode::<Vec<serde_json::Value>>(value)?
                    .into_iter()
                    .map(|v| typ.value_from_json(v))
                    .collect::<Result<_, _>>()?,
            ))),
            Type::Map(k, v) => Ok(Value::Map(MapValue::new_unchecked(
                k.clone(),
                v.clone(),
                decode::<HashMap<String, serde_json::Value>>(value)?
                    .into_iter()
                    .map(|(key, val)| {
                        Ok((
                            k.key_from_json(key)?,
                            v.value_from_json_unit(val, display_unit)?,
                        ))
                    })
                    .collect::<Result<_, DataError>>()?,
            ))),
            Type::Json => Ok(Value::Json(value)),
        }
    }

    #[cfg(feature = "dbschema")]
    pub fn dbschema(&self) -> DbSchema {
        match self {
            Type::BinaryString => ListSchema::new(IntegerSchema::new()).into(),
            Type::UnicodeString => StringSchema::new().into(),
            Type::Integer => IntegerSchema::new().into(),
            Type::Float => DoubleSchema::new().into(),
            Type::Quantity(_) => DoubleSchema::new().into(),
            Type::Enum(choices) => choices
                .iter()
                .fold(EnumSchema::new(), |schema, choice| {
                    schema.option(choice, UnitSchema::new())
                })
                .tag_string()
                .into(),
            Type::IntEnum(choices) => choices
                .values()
                .fold(EnumSchema::new(), |schema, choice| {
                    schema.option(choice, UnitSchema::new())
                })
                .tag_string()
                .into(),
            Type::Boolean => BoolSchema::new().into(),
            Type::Time => DateTimeSchema::new().into(),
            Type::Age => DoubleSchema::new().into(),
            Type::MacAddress => StringSchema::new().into(),
            Type::Ipv4Address => Ipv4Schema::new().into(),
            Type::Ipv6Address => Ipv6Schema::new().into(),
            Type::Option(t) => OptionSchema::new(t.dbschema()).into(),
            Type::Result(t, e) => EnumSchema::new()
                .option("ok", t.dbschema())
                .option("err", e.dbschema())
                .into(),
            Type::List(t) => ListSchema::new(t.dbschema()).into(),
            Type::Set(t) => SetSchema::new(t.dbschema()).into(),
            Type::Map(_, v) => DictionarySchema::new(v.dbschema()).into(),
            Type::Tuple(ts) => ts
                .iter()
                .enumerate()
                .fold(StructSchema::new(), |schema, (i, t)| {
                    schema.field(i.to_string(), t.dbschema())
                })
                .into(),
            Type::Json => JsonSchema::new().into(),
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::BinaryString => write!(f, "binarystring"),
            Type::UnicodeString => write!(f, "unicodestring"),
            Type::Integer => write!(f, "integer"),
            Type::Float => write!(f, "float"),
            Type::Quantity(d) => write!(f, "quantity({})", d),
            Type::Enum(cs) => write!(f, "enum({cs:?})"),
            Type::IntEnum(cs) => write!(f, "int_enum({cs:?})"),
            Type::Boolean => write!(f, "boolean"),
            Type::Time => write!(f, "time"),
            Type::Age => write!(f, "age"),
            Type::MacAddress => write!(f, "macaddr"),
            Type::Ipv4Address => write!(f, "ipaddr"),
            Type::Ipv6Address => write!(f, "ipv6addr"),
            Type::Option(t) => write!(f, "option({t})"),
            Type::Result(t, e) => write!(f, "result({t},{e})"),
            Type::List(t) => write!(f, "list({t})"),
            Type::Set(t) => write!(f, "set({t})"),
            Type::Map(k, v) => write!(f, "map({k},{v})"),
            Type::Tuple(ts) => write!(
                f,
                "tuple({})",
                ts.iter()
                    .map(|t| t.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Type::Json => write!(f, "json"),
        }
    }
}
