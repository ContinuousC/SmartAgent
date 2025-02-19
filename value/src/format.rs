/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use thiserror::Error;
use unit::{Quantity, UnitError};

use crate::HashableValue;

use super::options::FormatOpts;
use super::value::Value;

pub struct Format<'a, T>(pub(crate) &'a T);

impl Format<'_, Value> {
    pub(crate) fn fmt<T: std::fmt::Write>(
        &self,
        f: &mut T,
        opts: &FormatOpts,
    ) -> Result<(), FormatError> {
        match &self.0 {
            Value::BinaryString(bs) => {
                bs.iter().try_for_each(|c| write!(f, "{c:02x}"))?
            }
            Value::UnicodeString(s) => write!(f, "{s}")?,
            Value::Integer(n) => write!(f, "{n}")?,
            Value::Float(n) => match &opts.precision {
                Some(d) => write!(f, "{n:.0$}", *d as usize)?,
                None => write!(f, "{n}")?,
            },
            Value::Quantity(q) => {
                let q = match &opts.unit {
                    Some(unit) => q.convert(unit)?,
                    None => *q,
                };
                let Quantity(n, u) = match opts.autoscale {
                    true => q.autoscale()?,
                    false => q,
                };
                match &opts.precision {
                    Some(d) => write!(f, "{n:.0$} {u}", *d as usize)?,
                    None => write!(f, "{n} {u}")?,
                }
            }
            Value::Enum(v) => write!(f, "{}", v.get_value())?,
            Value::IntEnum(v) => write!(f, "{}", v.get_value_str())?,
            Value::Boolean(v) => write!(f, "{v}")?,
            Value::Time(t) => write!(f, "{}", t.to_rfc3339())?,
            Value::Age(d) => write!(f, "{}s", d.num_seconds())?, // TODO!
            Value::MacAddress(v) => write!(
                f,
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                v[0], v[1], v[2], v[3], v[4], v[5]
            )?,
            Value::Ipv4Address(v) => {
                write!(f, "{}.{}.{}.{}", v[0], v[1], v[2], v[3])?
            }
            Value::Ipv6Address(v) => write!(
                f,
                "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
            )?,
            Value::Option(v) => match v.get_value() {
                Some(v) => Format(v).fmt(f, opts)?,
                None => write!(f, "-")?,
            },
            Value::Result(v) => match v.get_value() {
                Ok(v) => Format(v).fmt(f, opts)?,
                Err(v) => Format(v).fmt(f, opts)?,
            },
            Value::List(v) => {
                let mut vs = v.get_values().iter();
                write!(f, "[")?;
                vs.next()
                    .into_iter()
                    .try_for_each(|v| Format(v).fmt(f, opts))?;
                vs.try_for_each(|v| {
                    write!(f, ", ")?;
                    Format(v).fmt(f, opts)
                })?;
                write!(f, "]")?
            }
            Value::Set(v) => {
                let mut vs = v.get_values().iter();
                write!(f, "[")?;
                vs.next()
                    .into_iter()
                    .try_for_each(|v| Format(v).fmt(f, opts))?;
                vs.try_for_each(|v| {
                    write!(f, ", ")?;
                    Format(v).fmt(f, opts)
                })?;
                write!(f, "]")?
            }
            Value::Map(v) => {
                let mut vs = v.get_values().iter();
                write!(f, "{{")?;
                vs.next().into_iter().try_for_each(|(k, v)| {
                    Format(k).fmt(f, opts)?;
                    write!(f, ": ")?;
                    Format(v).fmt(f, opts)
                })?;
                vs.try_for_each(|(k, v)| {
                    write!(f, ", ")?;
                    Format(k).fmt(f, opts)?;
                    write!(f, ": ")?;
                    Format(v).fmt(f, opts)
                })?;
                write!(f, "}}")?
            }
            Value::Tuple(vs) => {
                write!(f, "(")?;
                let mut vs = vs.iter();
                vs.next()
                    .into_iter()
                    .try_for_each(|v| Format(v).fmt(f, opts))?;
                vs.try_for_each(|v| {
                    write!(f, ", ")?;
                    Format(v).fmt(f, opts)
                })?;
                write!(f, ")")?
            }
            Value::Json(v) => {
                write!(f, "{}", serde_json::to_string(v).unwrap())?
            }
        };
        Ok(())
    }
}

impl Format<'_, HashableValue> {
    pub(crate) fn fmt<T: std::fmt::Write>(
        &self,
        f: &mut T,
        opts: &FormatOpts,
    ) -> Result<(), FormatError> {
        match &self.0 {
            HashableValue::BinaryString(bs) => {
                bs.iter().try_for_each(|c| write!(f, "{c:02x}"))?
            }
            HashableValue::UnicodeString(n) => write!(f, "{n}")?,
            HashableValue::Integer(n) => write!(f, "{n}")?,
            HashableValue::Enum(v) => write!(f, "{}", v.get_value())?,
            HashableValue::IntEnum(v) => write!(f, "{}", v.get_value_str())?,
            HashableValue::Boolean(v) => write!(f, "{v}")?,
            HashableValue::MacAddress(v) => write!(
                f,
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                v[0], v[1], v[2], v[3], v[4], v[5]
            )?,
            HashableValue::Ipv4Address(v) => {
                write!(f, "{}.{}.{}.{}", v[0], v[1], v[2], v[3])?
            }
            HashableValue::Ipv6Address(v) => write!(
                f,
                "{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
                v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7]
            )?,
            HashableValue::Option(v) => match v.get_value() {
                Some(v) => Format(v).fmt(f, opts)?,
                None => write!(f, "-")?,
            },
            HashableValue::Result(v) => match v.get_value() {
                Ok(v) => Format(v).fmt(f, opts)?,
                Err(v) => Format(v).fmt(f, opts)?,
            },
            HashableValue::List(v) => {
                let mut vs = v.get_values().iter();
                write!(f, "[")?;
                vs.next()
                    .into_iter()
                    .try_for_each(|v| Format(v).fmt(f, opts))?;
                vs.try_for_each(|v| {
                    write!(f, ", ")?;
                    Format(v).fmt(f, opts)
                })?;
                write!(f, "]")?
            }
            HashableValue::Tuple(vs) => {
                let mut vs = vs.iter();
                write!(f, "[")?;
                vs.next()
                    .into_iter()
                    .try_for_each(|v| Format(v).fmt(f, opts))?;
                vs.try_for_each(|v| {
                    write!(f, ", ")?;
                    Format(v).fmt(f, opts)
                })?;
                write!(f, "]")?
            }
        };
        Ok(())
    }
}

#[derive(Error, Debug)]
pub(crate) enum FormatError {
    #[error(transparent)]
    Unit(#[from] UnitError),
    #[error(transparent)]
    Fmt(#[from] std::fmt::Error),
}

impl FormatError {
    pub(crate) fn unwrap_fmt(self) -> UnitError {
        match self {
            FormatError::Fmt(_) => panic!(),
            FormatError::Unit(e) => e,
        }
    }
}
