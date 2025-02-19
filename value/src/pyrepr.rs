/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt::{self, Display};

use agent_utils::pyrepr::{PyBytes, PyUnicode};
use unit::Quantity;

use crate::{HashableValue, Value};

pub struct PyRepr<'a, T>(pub(crate) &'a T);

impl Display for PyRepr<'_, Value> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Value::BinaryString(v) => {
                write!(f, "{}", PyBytes(v))
            }
            Value::UnicodeString(v) => write!(f, "{}", PyUnicode(v)),
            Value::Integer(v) => write!(f, "{v}"),
            Value::Float(v) => write!(f, "{v}"),
            Value::Quantity(Quantity(v, u)) => {
                write!(f, "Quantity({v}, {})", PyUnicode(&u.to_string()))
            }
            Value::Time(v) => write!(f, "{}", PyUnicode(&v.to_string())),
            Value::Age(v) => write!(f, "{}", PyUnicode(&v.to_string())),
            Value::Enum(v) => write!(f, "{}", PyUnicode(v.get_value())),
            Value::IntEnum(v) => write!(f, "{}", PyUnicode(v.get_value_str())),
            Value::Boolean(v) => match v {
                true => write!(f, "True"),
                false => write!(f, "False"),
            },
            Value::MacAddress(v) => write!(
                f,
                "'{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}'",
                v[0], v[1], v[2], v[3], v[4], v[5]
            ),
            Value::Ipv4Address(v) => {
                write!(f, "'{}.{}.{}.{}'", v[0], v[1], v[2], v[3])
            }
            Value::Ipv6Address(v) => write!(
                f,
                "'{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}'",
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
            ),
            Value::Option(v) => match v.get_value() {
                Some(v) => write!(f, "{}", PyRepr(v)),
                None => write!(f, "None"),
            },
            Value::Result(v) => match v.get_value() {
                Ok(v) => write!(f, "{}", PyRepr(v)),
                Err(v) => write!(f, "{}", PyRepr(v)),
            },
            Value::List(v) => {
                write!(f, "[")?;
                let mut vs = v.get_values().iter();
                vs.next()
                    .into_iter()
                    .try_for_each(|v| write!(f, "{}", PyRepr(v)))?;
                vs.try_for_each(|v| write!(f, ", {}", PyRepr(v)))?;
                write!(f, "]")
            }
            Value::Set(v) => {
                write!(f, "{{")?;
                let mut vs = v.get_values().iter();
                vs.next()
                    .into_iter()
                    .try_for_each(|v| write!(f, "{}", PyRepr(v)))?;
                vs.try_for_each(|v| write!(f, ", {}", PyRepr(v)))?;
                write!(f, "}}")
            }
            Value::Map(v) => {
                write!(f, "{{")?;
                let mut vs = v.get_values().iter();
                vs.next().into_iter().try_for_each(|(k, v)| {
                    write!(f, "{}: {}", PyRepr(k), PyRepr(v))
                })?;
                vs.try_for_each(|(k, v)| {
                    write!(f, ", {}: {}", PyRepr(k), PyRepr(v))
                })?;
                write!(f, "}}")
            }
            Value::Tuple(vs) => {
                write!(f, "(")?;
                let mut vs = vs.iter();
                vs.next()
                    .into_iter()
                    .try_for_each(|v| write!(f, "{}", PyRepr(v)))?;
                vs.try_for_each(|v| write!(f, ", {}", PyRepr(v)))?;
                write!(f, ")")
            }
            Value::Json(v) => {
                write!(
                    f,
                    "json.loads({})",
                    PyUnicode(&serde_json::to_string(v).unwrap())
                )
            }
        }
    }
}

impl Display for PyRepr<'_, HashableValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            HashableValue::BinaryString(v) => write!(f, "{}", PyBytes(v)),
            HashableValue::UnicodeString(v) => write!(f, "{}", PyUnicode(v)),
            HashableValue::Integer(v) => write!(f, "{v}"),
            HashableValue::Enum(v) => write!(f, "{}", PyUnicode(v.get_value())),
            HashableValue::IntEnum(v) => {
                write!(f, "{}", PyUnicode(v.get_value_str()))
            }
            HashableValue::Boolean(v) => match v {
                true => write!(f, "True"),
                false => write!(f, "False"),
            },
            HashableValue::MacAddress(v) => write!(
                f,
                "'{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}'",
                v[0], v[1], v[2], v[3], v[4], v[5]
            ),
            HashableValue::Ipv4Address(v) => {
                write!(f, "'{}.{}.{}.{}'", v[0], v[1], v[2], v[3])
            }
            HashableValue::Ipv6Address(v) => write!(
                f,
                "'{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}'",
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
            ),
            HashableValue::Option(v) => match v.get_value() {
                Some(v) => write!(f, "{}", PyRepr(v)),
                None => write!(f, "None"),
            },
            HashableValue::Result(v) => match v.get_value() {
                Ok(v) => write!(f, "{}", PyRepr(v)),
                Err(v) => write!(f, "{}", PyRepr(v)),
            },
            HashableValue::List(v) => {
                write!(f, "[")?;
                let mut vs = v.get_values().iter();
                vs.next()
                    .into_iter()
                    .try_for_each(|v| write!(f, "{}", PyRepr(v)))?;
                vs.try_for_each(|v| write!(f, ", {}", PyRepr(v)))?;
                write!(f, "]")
            }
            HashableValue::Tuple(vs) => {
                write!(f, "(")?;
                let mut vs = vs.iter();
                vs.next()
                    .into_iter()
                    .try_for_each(|v| write!(f, "{}", PyRepr(v)))?;
                vs.try_for_each(|v| write!(f, ", {}", PyRepr(v)))?;
                write!(f, ")")
            }
        }
    }
}
