/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{BTreeMap, BTreeSet};
use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display};
use std::num::FpCategory;
use std::sync::Arc;

use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use unit::{Dimension, Quantity, Unit};

use crate::format::Format;
use crate::hashable::HashableType;
use crate::pyrepr::PyRepr;

use super::error::{Data, DataError};
use super::hashable::HashableValue;
use super::options::{FormatOpts, TypeOpts};
use super::types::Type;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum Value {
    BinaryString(Vec<u8>),
    #[serde(alias = "string", deserialize_with = "deserialize_unicode_string")]
    UnicodeString(String),
    Integer(i64),
    Float(f64),
    Quantity(Quantity),
    Enum(EnumValue),
    #[serde(rename = "int-enum")]
    IntEnum(IntEnumValue),
    Boolean(bool),
    Time(DateTime<Utc>),
    #[serde(with = "agent_serde::duration")]
    #[cfg_attr(feature = "schemars", schemars(with = "f64"))]
    Age(Duration),
    #[serde(rename = "macaddr")]
    MacAddress([u8; 6]),
    #[serde(rename = "ipv4addr")]
    #[serde(alias = "ipaddr")]
    Ipv4Address([u8; 4]),
    #[serde(rename = "ipv6addr")]
    Ipv6Address([u16; 8]),
    Option(OptionValue),
    Result(ResultValue),
    Tuple(Vec<Value>),
    List(ListValue),
    Set(SetValue),
    Map(MapValue),
    Json(serde_json::Value),
}

#[derive(
    Serialize, Deserialize, Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Hash,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct EnumValue(Arc<BTreeSet<String>>, String);

#[derive(
    Serialize, Deserialize, Debug, Eq, PartialEq, Ord, PartialOrd, Clone, Hash,
)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct IntEnumValue(
    #[serde(with = "agent_serde::arc_intkey_map")]
    #[cfg_attr(
        feature = "schemars",
        schemars(with = "BTreeMap<String, String>")
    )]
    Arc<BTreeMap<i64, String>>,
    i64,
);

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct OptionValue(Arc<Type>, Option<Box<Value>>);

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ResultValue(Arc<Type>, Arc<Type>, Result<Box<Value>, Box<Value>>);

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SetValue(Arc<HashableType>, HashSet<HashableValue>);

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct MapValue(
    Arc<HashableType>,
    Arc<Type>,
    HashMap<HashableValue, Value>,
);

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ListValue(Arc<Type>, Vec<Value>);

impl Value {
    pub fn get_type(&self) -> Type {
        match self {
            Value::BinaryString(_) => Type::BinaryString,
            Value::UnicodeString(_) => Type::UnicodeString,
            Value::Enum(v) => v.get_type(),
            Value::IntEnum(v) => v.get_type(),
            Value::Integer(_) => Type::Integer,
            Value::Float(_) => Type::Float,
            Value::Quantity(q) => Type::Quantity(q.1.dimension()),
            Value::Boolean(_) => Type::Boolean,
            Value::Time(_) => Type::Time,
            Value::Age(_) => Type::Age,
            Value::MacAddress(_) => Type::MacAddress,
            Value::Ipv4Address(_) => Type::Ipv4Address,
            Value::Ipv6Address(_) => Type::Ipv6Address,
            Value::Option(v) => v.get_type(),
            Value::Result(v) => v.get_type(),
            Value::List(v) => v.get_type(),
            Value::Set(v) => v.get_type(),
            Value::Map(v) => v.get_type(),
            Value::Tuple(ts) => {
                Type::Tuple(ts.iter().map(|t| t.get_type()).collect())
            }
            Value::Json(_) => Type::Json,
        }
    }

    pub fn as_float(self) -> Option<f64> {
        match self {
            Value::Integer(v) => Some(v as f64),
            Value::Float(v) => Some(v),
            _ => None,
        }
    }

    /// Equality regardless of meaning (so that eg. Nan == Nan).
    pub fn literal_eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (Value::Float(a), Value::Float(b)) => literal_float_eq(*a, *b),
            (
                Value::Quantity(Quantity(a, ua)),
                Value::Quantity(Quantity(b, ub)),
            ) => ua == ub && literal_float_eq(*a, *b),
            (
                Value::Option(OptionValue(s, a)),
                Value::Option(OptionValue(t, b)),
            ) => match (a, b) {
                (Some(a), Some(b)) => a.literal_eq(b),
                (None, None) => s == t,
                _ => false,
            },
            (
                Value::Result(ResultValue(_, _, a)),
                Value::Result(ResultValue(_, _, b)),
            ) => match (a, b) {
                (Ok(a), Ok(b)) => a.literal_eq(b),
                (Err(a), Err(b)) => a.literal_eq(b),
                _ => false,
            },
            (a, b) => a == b,
        }
    }

    pub fn cast_to(self, target: &Type) -> Data {
        self.cast_to_opts(target, &TypeOpts::default())
    }

    // Should agree with Type::castable_to!
    pub fn cast_to_opts(self, target: &Type, opts: &TypeOpts) -> Data {
        let source = self.get_type();
        match target == &source {
            true => Ok(self),
            false => match (target, self) {
                (Type::BinaryString, Value::UnicodeString(s)) => {
                    match &opts.strict_strings {
                        false => {
                            Ok(Value::BinaryString(s.into_bytes().to_vec()))
                        }
                        true => Err(DataError::TypeError(
                            "implicit casts between binary and \
							 unicode strings are disabled"
                                .to_string(),
                        )),
                    }
                }
                (Type::UnicodeString, Value::BinaryString(bs)) => {
                    match &opts.strict_strings {
                        false => Ok(Value::UnicodeString(
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
                    Type::Quantity(Dimension::Dimensionless),
                    Value::Integer(v),
                ) => Ok(Value::Quantity(Quantity::from_value(v as f64))),
                (Type::Quantity(Dimension::Dimensionless), Value::Float(v)) => {
                    Ok(Value::Quantity(Quantity::from_value(v)))
                }
                (Type::Float, Value::Integer(v)) => Ok(Value::Float(v as f64)),
                (Type::Option(t), Value::Option(OptionValue(_, v))) => {
                    match v {
                        Some(v) => Ok(Value::Option(OptionValue(
                            t.clone(),
                            Some(Box::new(v.cast_to_opts(t, opts)?)),
                        ))),
                        None => Ok(Value::Option(OptionValue(t.clone(), None))),
                    }
                }
                (Type::Result(t, e), Value::Result(ResultValue(_, _, v))) => {
                    match v {
                        Ok(v) => Ok(Value::Result(ResultValue(
                            t.clone(),
                            e.clone(),
                            Ok(Box::new(v.cast_to_opts(t, opts)?)),
                        ))),
                        Err(v) => Ok(Value::Result(ResultValue(
                            t.clone(),
                            e.clone(),
                            Err(Box::new(v.cast_to_opts(e, opts)?)),
                        ))),
                    }
                }
                (Type::Tuple(ts), Value::Tuple(vs)) if ts.len() == vs.len() => {
                    Ok(Value::Tuple(
                        ts.iter()
                            .zip(vs)
                            .map(|(t, v)| v.cast_to_opts(t, opts))
                            .collect::<Result<_, DataError>>()?,
                    ))
                }
                (Type::List(t), Value::List(ListValue(_, vs))) => {
                    Ok(Value::List(ListValue(
                        t.clone(),
                        vs.into_iter()
                            .map(|v| v.cast_to_opts(t, opts))
                            .collect::<Result<_, DataError>>()?,
                    )))
                }
                (Type::Set(t), Value::Set(SetValue(_, vs))) => {
                    Ok(Value::Set(SetValue(
                        t.clone(),
                        vs.into_iter()
                            .map(|v| v.cast_to_opts(t, opts))
                            .collect::<Result<_, DataError>>()?,
                    )))
                }
                (Type::Map(k, v), Value::Map(MapValue(_, _, m))) => {
                    Ok(Value::Map(MapValue::new_unchecked(
                        k.clone(),
                        v.clone(),
                        m.into_iter()
                            .map(|(kv, vv)| {
                                Ok((
                                    kv.cast_to_opts(k, opts)?,
                                    vv.cast_to_opts(v, opts)?,
                                ))
                            })
                            .collect::<Result<_, DataError>>()?,
                    )))
                }
                _ => Err(DataError::TypeError(format!(
                    "expected {}, got {}",
                    target, source
                ))),
            },
        }
    }

    // Used by the to_string function in expr.
    pub fn into_string(self) -> Result<String, DataError> {
        self.format(&FormatOpts::default())
    }

    // Generate a user-readable representation of the value.
    pub fn format(&self, opts: &FormatOpts) -> Result<String, DataError> {
        let mut s = String::new();
        Format(self).fmt(&mut s, opts).map_err(|e| e.unwrap_fmt())?;
        Ok(s)
    }

    // Note: must agree with Type::dbschema!
    pub fn to_json_value(&self) -> Option<serde_json::Value> {
        self.to_json_value_res().ok()
    }

    pub fn to_json_value_res(
        &self,
    ) -> std::result::Result<serde_json::Value, String> {
        self.to_json_value_unit(None)
    }

    pub fn to_json_value_unit(
        &self,
        display_unit: Option<Unit>,
    ) -> std::result::Result<serde_json::Value, String> {
        Ok(match self {
            Value::BinaryString(v) => serde_json::Value::Array(
                v.iter()
                    .map(|b| {
                        Ok(serde_json::Value::Number(
                            serde_json::Number::from_f64(*b as f64)
                                .ok_or_else(|| "invalid byte!?".to_string())?,
                        ))
                    })
                    .collect::<std::result::Result<Vec<_>, String>>()?,
            ),
            Value::UnicodeString(v) => serde_json::Value::String(v.to_string()),
            Value::Integer(v) => serde_json::Value::Number(
                serde_json::Number::from_f64(*v as f64)
                    .ok_or_else(|| "invalid integer!?".to_string())?,
            ),
            Value::Float(v) => serde_json::Value::Number(
                serde_json::Number::from_f64(*v)
                    .ok_or_else(|| "NaN".to_string())?,
            ),
            Value::Quantity(v) => {
                let v = match display_unit {
                    Some(u) => {
                        v.convert(&u.normalize()).map_err(|e| e.to_string())?
                    }
                    None => v.normalize().map_err(|e| e.to_string())?,
                };
                serde_json::Value::Number(
                    serde_json::Number::from_f64(v.0)
                        .ok_or_else(|| "NaN".to_string())?,
                )
            }
            Value::Enum(EnumValue(_, v)) => {
                serde_json::Value::String(v.to_string())
            }
            Value::IntEnum(IntEnumValue(cs, v)) => serde_json::json!(cs
                .get(v)
                .ok_or_else(|| format!("invalid choice: {}", v))?),
            Value::Boolean(v) => serde_json::Value::Bool(*v),
            Value::Time(v) => serde_json::Value::String(
                v.to_rfc3339_opts(SecondsFormat::AutoSi, true),
            ),
            Value::Age(v) => serde_json::Value::Number(
                serde_json::Number::from_f64(
                    v.num_milliseconds() as f64 / 1000.0,
                )
                .ok_or_else(|| "invalid age value".to_string())?,
            ),
            Value::MacAddress(v) => serde_json::Value::String(
                v.iter()
                    .map(|i| format!("{:02x}", i))
                    .collect::<Vec<_>>()
                    .join(":"),
            ),
            Value::Ipv4Address(v) => serde_json::Value::String(
                v.iter()
                    .map(|i| format!("{}", i))
                    .collect::<Vec<_>>()
                    .join("."),
            ),
            Value::Ipv6Address(v) => serde_json::Value::String(
                v.iter()
                    .map(|i| format!("{:x}", i))
                    .collect::<Vec<_>>()
                    .join(":"),
            ),
            Value::Option(OptionValue(_, v)) => match v {
                Some(v) => v.to_json_value_unit(display_unit)?,
                None => serde_json::Value::Null,
            },
            Value::Result(ResultValue(_, _, v)) => match v {
                Ok(v) => {
                    serde_json::json!({"ok": v.to_json_value_unit(display_unit)?})
                }
                Err(e) => {
                    serde_json::json!({"err": e.to_json_value_unit(display_unit)?})
                }
            },
            Value::List(ListValue(_, vs)) => serde_json::Value::Array(
                vs.iter()
                    .map(|v| v.to_json_value_unit(display_unit))
                    .collect::<Result<_, String>>()?,
            ),
            Value::Set(SetValue(_, vs)) => serde_json::Value::Array(
                vs.iter()
                    .map(|v| v.to_json_value())
                    .collect::<Result<_, String>>()?,
            ),
            Value::Map(MapValue(_, _, vs)) => serde_json::Value::Object(
                vs.iter()
                    .map(|(k, v)| {
                        Ok((
                            k.to_json_key()?,
                            v.to_json_value_unit(display_unit)?,
                        ))
                    })
                    .collect::<Result<_, String>>()?,
            ),
            Value::Tuple(vs) => serde_json::Value::Object(
                vs.iter()
                    .enumerate()
                    .try_fold::<_, _, std::result::Result<_, String>>(
                        serde_json::Map::new(),
                        |mut map, (i, v)| {
                            map.insert(
                                format!("{}", i),
                                v.to_json_value_res()?,
                            );
                            Ok(map)
                        },
                    )?,
            ),
            Value::Json(v) => v.clone(),
        })
    }

    /// Convert to a JSON value that sorts correctly. This is used
    /// for sorting in tabulator.
    pub fn to_sortable_json_value(&self) -> serde_json::Value {
        self.to_json_value_res().unwrap_or(serde_json::Value::Null)
    }

    pub fn py_repr(&self) -> PyRepr<Self> {
        PyRepr(self)
    }
}

impl EnumValue {
    pub fn new(
        choices: Arc<BTreeSet<String>>,
        value: String,
    ) -> Result<Self, DataError> {
        match choices.contains(&value) {
            true => Ok(Self(choices, value)),
            false => Err(DataError::InvalidChoice(value)),
        }
    }

    pub fn deconstruct(self) -> (Arc<BTreeSet<String>>, String) {
        (self.0, self.1)
    }

    pub fn get_type(&self) -> Type {
        Type::Enum(self.0.clone())
    }

    pub fn choices(&self) -> &Arc<BTreeSet<String>> {
        &self.0
    }

    pub fn get_value(&self) -> &str {
        &self.1
    }
}

impl IntEnumValue {
    pub fn new(
        choices: Arc<BTreeMap<i64, String>>,
        value: i64,
    ) -> Result<Self, DataError> {
        match choices.contains_key(&value) {
            true => Ok(Self(choices, value)),
            false => Err(DataError::InvalidIntChoice(value)),
        }
    }

    pub fn deconstruct(self) -> (Arc<BTreeMap<i64, String>>, i64) {
        (self.0, self.1)
    }

    pub fn get_type(&self) -> Type {
        Type::IntEnum(self.0.clone())
    }

    pub fn choices(&self) -> &Arc<BTreeMap<i64, String>> {
        &self.0
    }

    pub fn get_value_int(&self) -> i64 {
        self.1
    }

    pub fn get_value_str(&self) -> &str {
        self.0.get(&self.1).unwrap()
    }
}

impl OptionValue {
    pub fn new(
        typ: Arc<Type>,
        value: Option<Value>,
    ) -> Result<Self, DataError> {
        match value {
            Some(value) => match &value.get_type() == typ.as_ref() {
                true => Ok(Self(typ, Some(Box::new(value)))),
                false => Err(DataError::InvalidOptionValue),
            },
            None => Ok(Self(typ, None)),
        }
    }

    pub(crate) fn new_unchecked(typ: Arc<Type>, value: Option<Value>) -> Self {
        Self(typ, value.map(Box::new))
    }

    pub fn deconstruct(self) -> (Arc<Type>, Option<Value>) {
        (
            self.0,
            self.1.as_ref().map(|v| v.as_ref().clone()), /* Box::into_inner */
        )
    }

    pub fn get_type(&self) -> Type {
        Type::Option(self.0.clone())
    }

    pub fn get_value(&self) -> Option<&Value> {
        self.1.as_ref().map(|v| v.as_ref())
    }
}

impl ResultValue {
    pub fn new(
        ok: Arc<Type>,
        err: Arc<Type>,
        value: Result<Value, Value>,
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

    pub(crate) fn new_unchecked(
        ok: Arc<Type>,
        err: Arc<Type>,
        value: Result<Value, Value>,
    ) -> Self {
        Self(ok, err, value.map(Box::new).map_err(Box::new))
    }

    pub fn get_type(&self) -> Type {
        Type::Result(self.0.clone(), self.1.clone())
    }

    pub fn get_value(&self) -> Result<&Value, &Value> {
        self.2.as_ref().map(|v| v.as_ref()).map_err(|v| v.as_ref())
    }

    pub fn deconstruct(self) -> (Arc<Type>, Arc<Type>, Result<Value, Value>) {
        (
            self.0,
            self.1,
            self.2
                .as_ref()
                .map(|v| v.as_ref().clone() /* Box::into_inner */)
                .map_err(|v| v.as_ref().clone()),
        )
    }
}

impl ListValue {
    pub fn new(typ: Arc<Type>, value: Vec<Value>) -> Result<Self, DataError> {
        match value.iter().all(|v| &v.get_type() == typ.as_ref()) {
            true => Ok(Self(typ, value)),
            false => Err(DataError::InvalidListValue),
        }
    }

    pub(crate) fn new_unchecked(typ: Arc<Type>, value: Vec<Value>) -> Self {
        Self(typ, value)
    }

    pub fn get_type(&self) -> Type {
        Type::List(self.0.clone())
    }

    pub fn get_values(&self) -> &Vec<Value> {
        &self.1
    }
}

impl SetValue {
    pub fn new(
        typ: Arc<HashableType>,
        value: HashSet<HashableValue>,
    ) -> Result<Self, DataError> {
        match value.iter().all(|v| &v.get_type() == typ.as_ref()) {
            true => Ok(Self(typ, value)),
            false => Err(DataError::InvalidSetValue),
        }
    }

    pub(crate) fn new_unchecked(
        typ: Arc<HashableType>,
        value: HashSet<HashableValue>,
    ) -> Self {
        Self(typ, value)
    }

    pub fn get_type(&self) -> Type {
        Type::Set(self.0.clone())
    }

    pub fn get_values(&self) -> &HashSet<HashableValue> {
        &self.1
    }
}

impl MapValue {
    pub fn new(
        k: Arc<HashableType>,
        v: Arc<Type>,
        value: HashMap<HashableValue, Value>,
    ) -> Result<Self, DataError> {
        match value.iter().all(|(kv, vv)| {
            &kv.get_type() == k.as_ref() && &vv.get_type() == v.as_ref()
        }) {
            true => Ok(Self(k, v, value)),
            false => Err(DataError::InvalidSetValue),
        }
    }

    pub(crate) fn new_unchecked(
        k: Arc<HashableType>,
        v: Arc<Type>,
        value: HashMap<HashableValue, Value>,
    ) -> Self {
        Self(k, v, value)
    }

    pub fn get_type(&self) -> Type {
        Type::Set(self.0.clone())
    }

    pub fn get_values(&self) -> &HashMap<HashableValue, Value> {
        &self.2
    }
}

fn literal_float_eq(a: f64, b: f64) -> bool {
    match (a.classify(), b.classify()) {
        (FpCategory::Normal, FpCategory::Normal) => a == b,
        (FpCategory::Subnormal, FpCategory::Subnormal) => a == b,
        (c, d) => c == d,
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::BinaryString(v) => {
                write!(f, "{:?}", String::from_utf8_lossy(v))
            }
            Value::UnicodeString(v) => write!(f, "{v:?}"),
            Value::Integer(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Quantity(v) => write!(f, "{v}"),
            Value::Time(v) => write!(f, "{v}"),
            Value::Age(v) => write!(f, "{v}"),
            Value::Enum(v) => write!(f, "{}", v.get_value()),
            Value::IntEnum(v) => {
                write!(f, "{}", v.get_value_str())
            }
            Value::Boolean(v) => write!(f, "{v}"),
            Value::MacAddress(v) => write!(
                f,
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                v[0], v[1], v[2], v[3], v[4], v[5]
            ),
            Value::Ipv4Address(v) => {
                write!(f, "{}.{}.{}.{}", v[0], v[1], v[2], v[3])
            }
            Value::Ipv6Address(v) => write!(
                f,
                "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
            ),
            Value::Option(v) => match v.get_value() {
                None => write!(f, "None"),
                Some(v) => write!(f, "{v}"),
            },
            Value::Result(v) => match v.get_value() {
                Ok(v) => write!(f, "Ok({v})"),
                Err(e) => write!(f, "Err({e})"),
            },
            Value::List(v) => {
                write!(f, "[")?;
                let mut vs = v.get_values().iter();
                vs.next().into_iter().try_for_each(|v| write!(f, "{v}"))?;
                vs.try_for_each(|v| write!(f, ", {v}"))?;
                write!(f, "]")
            }
            Value::Set(v) => {
                write!(f, "{{")?;
                let mut vs = v.get_values().iter();
                vs.next().into_iter().try_for_each(|v| write!(f, "{v}"))?;
                vs.try_for_each(|v| write!(f, ", {v}"))?;
                write!(f, "}}")
            }
            Value::Map(v) => {
                write!(f, "{{")?;
                let mut vs = v.get_values().iter();
                vs.next()
                    .into_iter()
                    .try_for_each(|(k, v)| write!(f, "{k}: {v}"))?;
                vs.try_for_each(|(k, v)| write!(f, ", {k}: {v}"))?;
                write!(f, "}}")
            }
            Value::Tuple(vs) => {
                write!(f, "(")?;
                let mut vs = vs.iter();
                vs.next().into_iter().try_for_each(|v| write!(f, "{v}"))?;
                vs.try_for_each(|v| write!(f, ", {v}"))?;
                write!(f, ")")
            }
            Value::Json(v) => {
                write!(f, "{}", serde_json::to_string(&v).unwrap())
            }
        }
    }
}

// Backward-compatible unicode string deserialization:
// the value can either be a string or an array of bytes.
fn deserialize_unicode_string<'de, D>(
    deserializer: D,
) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    match UnicodeStringCompat::deserialize(deserializer)? {
        UnicodeStringCompat::String(s) => Ok(s),
        UnicodeStringCompat::Bytes(bs) => {
            Ok(String::from_utf8_lossy(&bs).to_string())
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum UnicodeStringCompat {
    String(String),
    Bytes(Vec<u8>),
}
