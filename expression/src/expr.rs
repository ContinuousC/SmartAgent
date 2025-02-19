/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt::{self, Display};
use std::ops::RangeInclusive;

use agent_utils::pyrepr::PyUnicode;
use chrono::{Duration, NaiveDate, TimeZone, Utc};
use derivative::Derivative;
use dynfmt::{python::PythonFormat, Format};
use regex::Regex;
use serde::{Deserialize, Serialize};

use unit::{Dimension, FracPrefix, Quantity, TimeUnit, Unit};
use value::{Data, DataError, NumericTypePair, NumericValuePair, Type, Value};

use crate::options::EvalOpts;

use super::error::EvalError;
use super::eval::EvalCell;
use super::parser::parse_expr;

#[derive(Serialize, Deserialize, Clone, Debug, Derivative)]
#[derivative(PartialEq, Eq)]
pub enum Expr {
    // Primitives
    Data,

    Literal(#[derivative(PartialEq(compare_with = "Value::literal_eq"))] Value),
    Variable(String),

    // Boolean operators
    Or(Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),

    // Comparison operators
    Le(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    Gt(Box<Expr>, Box<Expr>),
    Ge(Box<Expr>, Box<Expr>),

    // Numeric operators
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),

    // Unit specification
    Quantity(Box<Expr>, Unit),
    Convert(Box<Expr>, Unit),

    // General functions
    Fallback(Box<Expr>, Box<Expr>),

    // Type conversions
    FromUtf8(Box<Expr>),
    FromUtf8Lossy(Box<Expr>),
    // FromUtf16(Box<Expr>),
    // FromUtf16Lossy(Box<Expr>),
    ToBinary(Box<Expr>),
    ParseInt(Box<Expr>),
    ParseFloat(Box<Expr>),
    ParseMacBin(Box<Expr>),
    ParseIpv4Bin(Box<Expr>),
    ParseIpv6Bin(Box<Expr>),
    AgeFromSeconds(Box<Expr>),
    EnumValue(Box<Expr>),
    UnwrapError(Box<Expr>),

    // String functions
    Concat(Box<Expr>, Box<Expr>),
    Format(String, Box<Expr>),
    ToString(Box<Expr>),
    RegSubst(
        Box<Expr>,
        #[derivative(PartialEq(compare_with = "agent_serde::regex::compare"))]
        #[serde(with = "agent_serde::regex")]
        Regex,
        String,
    ),
    SubStr(Box<Expr>, Box<Expr>, Box<Expr>),
    HexStr(Box<Expr>),
    SHA1(Box<Expr>),
    MD5(Box<Expr>),

    // Validation
    NotEmpty(Box<Expr>),

    // Numeric Functions
    Log(Box<Expr>, Box<Expr>),
    Sign(Box<Expr>),
    Abs(Box<Expr>),

    // Bit field
    BitsLE(Box<Expr>, Box<Expr>, Box<Expr>),
    BitsBE(Box<Expr>, Box<Expr>, Box<Expr>),

    // MP-specific
    UnpackTime(Box<Expr>),
}

pub struct PyRepr<'a>(&'a Expr);

impl Expr {
    pub fn parse(input: &str) -> Result<Self, EvalError> {
        parse_expr(input)
    }

    pub fn eval(&self, data: Option<&Data>) -> Result<Value, EvalError> {
        self.eval_opts(data, &EvalOpts::default())
    }

    pub fn check(&self, data: Option<&Type>) -> Result<Type, EvalError> {
        self.check_opts(data, &EvalOpts::default())
    }

    pub fn eval_opts(
        &self,
        data: Option<&Data>,
        opts: &EvalOpts,
    ) -> Result<Value, EvalError> {
        self.eval_in_row_opts(None, data, opts)
    }

    pub fn check_opts(
        &self,
        data: Option<&Type>,
        opts: &EvalOpts,
    ) -> Result<Type, EvalError> {
        self.check_in_row_opts(None, data, opts)
    }

    pub fn eval_in_row<'a>(
        &self,
        vars: Option<&'a HashMap<&'a str, EvalCell<'a, Data, Value>>>,
        data: Option<&Data>,
    ) -> Result<Value, EvalError> {
        self.eval_in_row_opts(vars, data, &EvalOpts::default())
    }

    pub fn eval_in_row_opts<'a>(
        &self,
        vars: Option<&'a HashMap<&'a str, EvalCell<'a, Data, Value>>>,
        data: Option<&Data>,
        opts: &EvalOpts,
    ) -> Result<Value, EvalError> {
        match self {
            Self::Literal(v) => Ok(v.clone()),

            Self::Data => match data {
                Some(v) => Ok(v.clone()?),
                None => Err(EvalError::DataError(DataError::Missing)),
            },

            Self::Variable(n) => match vars.and_then(|v| v.get(n.as_str())) {
                Some(c) => {
                    c.eval(|e, d| e.eval_in_row_opts(vars, d, opts)).map_err(
                        |e| EvalError::VariableError(n.clone(), Box::new(e)),
                    )
                }
                None => Err(EvalError::MissingVariable(n.clone())),
            },

            Self::Or(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::Boolean(v1), Value::Boolean(v2)) => {
                        Ok(Value::Boolean(v1 || v2))
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid types for boolean or",
                    )),
                }
            }

            Self::And(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::Boolean(v1), Value::Boolean(v2)) => {
                        Ok(Value::Boolean(v1 && v2))
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid types for boolean and",
                    )),
                }
            }

            Self::Not(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::Boolean(v) => Ok(Value::Boolean(!v)),
                _ => Err(EvalError::TypeError("invalid types for boolean not")),
            },

            Self::Gt(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::UnicodeString(v1), Value::UnicodeString(v2)) => {
                        Ok(Value::Boolean(v1 > v2))
                    }
                    (Value::BinaryString(v1), Value::BinaryString(v2)) => {
                        Ok(Value::Boolean(v1 > v2))
                    }
                    (Value::Integer(v1), Value::Integer(v2)) => {
                        Ok(Value::Boolean(v1 > v2))
                    }
                    (Value::Float(v1), Value::Float(v2)) => {
                        Ok(Value::Boolean(v1 > v2))
                    }
                    (Value::Time(t1), Value::Time(t2)) => {
                        Ok(Value::Boolean(t1 > t2))
                    }
                    (Value::Age(d1), Value::Age(d2)) => {
                        Ok(Value::Boolean(d1 > d2))
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid types for comparison operator",
                    )),
                }
            }

            Self::Ge(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::BinaryString(v1), Value::BinaryString(v2)) => {
                        Ok(Value::Boolean(v1 >= v2))
                    }
                    (Value::UnicodeString(v1), Value::UnicodeString(v2)) => {
                        Ok(Value::Boolean(v1 >= v2))
                    }
                    (Value::Integer(v1), Value::Integer(v2)) => {
                        Ok(Value::Boolean(v1 >= v2))
                    }
                    (Value::Float(v1), Value::Float(v2)) => {
                        Ok(Value::Boolean(v1 >= v2))
                    }
                    (Value::Time(t1), Value::Time(t2)) => {
                        Ok(Value::Boolean(t1 >= t2))
                    }
                    (Value::Age(d1), Value::Age(d2)) => {
                        Ok(Value::Boolean(d1 >= d2))
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid types for comparison operator",
                    )),
                }
            }

            Self::Eq(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::BinaryString(v1), Value::BinaryString(v2)) => {
                        Ok(Value::Boolean(v1 == v2))
                    }
                    (Value::UnicodeString(v1), Value::UnicodeString(v2)) => {
                        Ok(Value::Boolean(v1 == v2))
                    }
                    (Value::Integer(v1), Value::Integer(v2)) => {
                        Ok(Value::Boolean(v1 == v2))
                    }
                    (Value::Float(v1), Value::Float(v2)) => {
                        Ok(Value::Boolean(v1 == v2))
                    }
                    (Value::Time(t1), Value::Time(t2)) => {
                        Ok(Value::Boolean(t1 == t2))
                    }
                    (Value::Age(d1), Value::Age(d2)) => {
                        Ok(Value::Boolean(d1 == d2))
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid types for comparison operator",
                    )),
                }
            }

            Self::Ne(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::BinaryString(v1), Value::BinaryString(v2)) => {
                        Ok(Value::Boolean(v1 != v2))
                    }
                    (Value::UnicodeString(v1), Value::UnicodeString(v2)) => {
                        Ok(Value::Boolean(v1 != v2))
                    }
                    (Value::Integer(v1), Value::Integer(v2)) => {
                        Ok(Value::Boolean(v1 != v2))
                    }
                    (Value::Float(v1), Value::Float(v2)) => {
                        Ok(Value::Boolean(v1 != v2))
                    }
                    (Value::Time(t1), Value::Time(t2)) => {
                        Ok(Value::Boolean(t1 != t2))
                    }
                    (Value::Age(d1), Value::Age(d2)) => {
                        Ok(Value::Boolean(d1 != d2))
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid types for comparison operator",
                    )),
                }
            }

            Self::Le(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::BinaryString(v1), Value::BinaryString(v2)) => {
                        Ok(Value::Boolean(v1 <= v2))
                    }
                    (Value::UnicodeString(v1), Value::UnicodeString(v2)) => {
                        Ok(Value::Boolean(v1 <= v2))
                    }
                    (Value::Integer(v1), Value::Integer(v2)) => {
                        Ok(Value::Boolean(v1 <= v2))
                    }
                    (Value::Float(v1), Value::Float(v2)) => {
                        Ok(Value::Boolean(v1 <= v2))
                    }
                    (Value::Time(t1), Value::Time(t2)) => {
                        Ok(Value::Boolean(t1 <= t2))
                    }
                    (Value::Age(d1), Value::Age(d2)) => {
                        Ok(Value::Boolean(d1 <= d2))
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid types for comparison operator",
                    )),
                }
            }

            Self::Lt(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::BinaryString(v1), Value::BinaryString(v2)) => {
                        Ok(Value::Boolean(v1 < v2))
                    }
                    (Value::UnicodeString(v1), Value::UnicodeString(v2)) => {
                        Ok(Value::Boolean(v1 < v2))
                    }
                    (Value::Integer(v1), Value::Integer(v2)) => {
                        Ok(Value::Boolean(v1 < v2))
                    }
                    (Value::Float(v1), Value::Float(v2)) => {
                        Ok(Value::Boolean(v1 < v2))
                    }
                    (Value::Time(t1), Value::Time(t2)) => {
                        Ok(Value::Boolean(t1 < t2))
                    }
                    (Value::Age(d1), Value::Age(d2)) => {
                        Ok(Value::Boolean(d1 < d2))
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid types for comparison operator",
                    )),
                }
            }

            Self::Add(e1, e2) => match (
                e1.eval_in_row_opts(vars, data, opts)?,
                e2.eval_in_row_opts(vars, data, opts)?,
            ) {
                (Value::Time(t), Value::Age(d))
                | (Value::Age(d), Value::Time(t)) => Ok(Value::Time(
                    t.checked_add_signed(d).ok_or(EvalError::TimeOverflow)?,
                )),
                (Value::Age(d1), Value::Age(d2)) => Ok(Value::Age(
                    d1.checked_add(&d2).ok_or(EvalError::TimeOverflow)?,
                )),
                (v1, v2) => match NumericValuePair::from(v1, v2) {
                    Some(NumericValuePair::Integer(v1, v2)) => {
                        match v1.checked_add(v2) {
                            Some(v) => Ok(Value::Integer(v)),
                            None => Err(EvalError::IntegerOverflow),
                        }
                    }
                    Some(NumericValuePair::Float(v1, v2)) => {
                        Ok(Value::Float(v1 + v2))
                    }
                    Some(NumericValuePair::Quantity(v1, v2)) => {
                        Ok(Value::Quantity((v1 + v2)?))
                    }
                    None => {
                        Err(EvalError::TypeError("invalid types for addition"))
                    }
                },
            },

            Self::Sub(e1, e2) => match (
                e1.eval_in_row_opts(vars, data, opts)?,
                e2.eval_in_row_opts(vars, data, opts)?,
            ) {
                (Value::Time(t), Value::Age(d)) => Ok(Value::Time(
                    t.checked_sub_signed(d).ok_or(EvalError::TimeOverflow)?,
                )),
                (Value::Time(t1), Value::Time(t2)) => {
                    Ok(Value::Age(t1.signed_duration_since(t2)))
                }
                (Value::Age(d1), Value::Age(d2)) => Ok(Value::Age(
                    d1.checked_sub(&d2).ok_or(EvalError::TimeOverflow)?,
                )),
                (v1, v2) => match NumericValuePair::from(v1, v2) {
                    Some(NumericValuePair::Integer(v1, v2)) => {
                        match v1.checked_sub(v2) {
                            Some(v) => Ok(Value::Integer(v)),
                            None => Err(EvalError::IntegerOverflow),
                        }
                    }
                    Some(NumericValuePair::Float(v1, v2)) => {
                        Ok(Value::Float(v1 - v2))
                    }
                    Some(NumericValuePair::Quantity(v1, v2)) => {
                        Ok(Value::Quantity((v1 - v2)?))
                    }
                    None => Err(EvalError::TypeError(
                        "invalid types for subtraction",
                    )),
                },
            },

            Self::Mul(e1, e2) => match NumericValuePair::from(
                e1.eval_in_row_opts(vars, data, opts)?,
                e2.eval_in_row_opts(vars, data, opts)?,
            ) {
                Some(NumericValuePair::Integer(v1, v2)) => {
                    match v1.checked_mul(v2) {
                        Some(v) => Ok(Value::Integer(v)),
                        None => Err(EvalError::IntegerOverflow),
                    }
                }
                Some(NumericValuePair::Float(v1, v2)) => {
                    Ok(Value::Float(v1 * v2))
                }
                Some(NumericValuePair::Quantity(v1, v2)) => {
                    Ok(Value::Quantity((v1 * v2)?))
                }
                None => Err(EvalError::TypeError(
                    "invalid types for multiplication",
                )),
            },

            Self::Div(e1, e2) => match NumericValuePair::from(
                e1.eval_in_row_opts(vars, data, opts)?,
                e2.eval_in_row_opts(vars, data, opts)?,
            ) {
                Some(NumericValuePair::Integer(v1, v2)) => {
                    Ok(Value::Float(v1 as f64 / v2 as f64))
                }
                Some(NumericValuePair::Float(v1, v2)) => {
                    Ok(Value::Float(v1 / v2))
                }
                Some(NumericValuePair::Quantity(v1, v2)) => {
                    Ok(Value::Quantity((v1 / v2)?))
                }
                None => Err(EvalError::TypeError("invalid types for division")),
            },

            Self::Pow(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::Integer(v1), Value::Integer(v2)) => {
                        Ok(Value::Float((v1 as f64).powi(v2 as i32)))
                    }
                    (Value::Float(v1), Value::Integer(v2)) => {
                        Ok(Value::Float(v1.powi(v2 as i32)))
                    }
                    (Value::Integer(v1), Value::Float(v2)) => {
                        Ok(Value::Float((v1 as f64).powf(v2)))
                    }
                    (Value::Float(v1), Value::Float(v2)) => {
                        Ok(Value::Float(v1.powf(v2)))
                    }
                    (Value::Quantity(v1), Value::Integer(v2)) => {
                        Ok(Value::Quantity(v1.powi(v2 as i32)?))
                    }
                    _ => Err(EvalError::TypeError("invalid types for power")),
                }
            }

            Self::Neg(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::Integer(v) => Ok(Value::Integer(-v)),
                Value::Float(v) => Ok(Value::Float(-v)),
                Value::Quantity(Quantity(v, u)) => {
                    Ok(Value::Quantity(Quantity(-v, u)))
                }
                Value::Age(d) => Ok(Value::Age(-d)),
                _ => Err(EvalError::TypeError("invalid type for negation")),
            },

            Self::Log(e1, e2) => match NumericValuePair::from(
                e1.eval_in_row_opts(vars, data, opts)?,
                e2.eval_in_row_opts(vars, data, opts)?,
            ) {
                Some(NumericValuePair::Integer(b, v)) => {
                    Ok(Value::Float((v as f64).log(b as f64)))
                }
                Some(NumericValuePair::Float(b, v)) => {
                    Ok(Value::Float(v.log(b)))
                }
                _ => Err(EvalError::TypeError("invalid types for logarithm")),
            },

            Self::Abs(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::Integer(v) => Ok(Value::Integer(v.abs())),
                Value::Float(v) => Ok(Value::Float(v.abs())),
                Value::Quantity(Quantity(v, u)) => {
                    Ok(Value::Quantity(Quantity(v.abs(), u)))
                }
                _ => Err(EvalError::TypeError("invalid type for abs")),
            },

            Self::Sign(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::Integer(v) => {
                    Ok(Value::Integer(if v >= 0 { 1 } else { -1 }))
                }
                Value::Float(v) => {
                    Ok(Value::Integer(if v >= 0.0 { 1 } else { -1 }))
                }
                Value::Quantity(Quantity(v, _)) => {
                    Ok(Value::Integer(if v >= 0.0 { 1 } else { -1 }))
                }
                _ => Err(EvalError::TypeError("invalid type for abs")),
            },

            Self::BitsBE(e1, e2, e3) => match (
                e1.eval_in_row_opts(vars, data, opts)?,
                e2.eval_in_row_opts(vars, data, opts)?,
                e3.eval_in_row_opts(vars, data, opts)?,
            ) {
                (
                    Value::BinaryString(data),
                    Value::Integer(from),
                    Value::Integer(len),
                ) => match from >= 0 && len >= 0 && len <= 62 {
                    true => match data
                        .get(from as usize / 8..((from + len + 7) as usize) / 8)
                    {
                        Some(vs) => Ok(Value::Integer(
                            vs.iter()
                                .fold((0, from % 8, len), |(r, o, n), v| {
                                    (
                                        r << n.min(8)
                                            | *v as i64 >> (8 - o - n).max(0)
                                                & 0xff >> o,
                                        0,
                                        n + o - 8,
                                    )
                                })
                                .0,
                        )),
                        None => Err(EvalError::OutOfBounds),
                    },
                    false => Err(EvalError::OutOfBounds),
                },
                _ => Err(EvalError::TypeError(
                    "invalid argument types for 'bits_be' function \
					       (expected: binarystring, int, int)",
                )),
            },

            Self::BitsLE(e1, e2, e3) => match (
                e1.eval_in_row_opts(vars, data, opts)?,
                e2.eval_in_row_opts(vars, data, opts)?,
                e3.eval_in_row_opts(vars, data, opts)?,
            ) {
                (
                    Value::BinaryString(data),
                    Value::Integer(from),
                    Value::Integer(len),
                ) => match from >= 0 && len >= 0 && len <= 62 {
                    true => match data
                        .get(from as usize / 8..((from + len + 7) as usize) / 8)
                    {
                        Some(vs) => Ok(Value::Integer(
                            vs.iter()
                                .fold((0, 0), |(r, i), v| {
                                    (
                                        r | if i < len {
                                            ((*v << from % 8) as i64) << i
                                                >> (i - len + 8).max(0)
                                        } else {
                                            0
                                        } | if i > 0 {
                                            (*v as i64) >> 8 - from % 8 << i - 8
                                        } else {
                                            0
                                        },
                                        i + 8,
                                    )
                                })
                                .0,
                        )),
                        None => Err(EvalError::OutOfBounds),
                    },
                    false => Err(EvalError::OutOfBounds),
                },
                _ => Err(EvalError::TypeError(
                    "invalid argument types for 'bits_le' function \
					       (expected: string, int, int)",
                )),
            },

            Self::Fallback(e1, e2) => {
                match e1.eval_in_row_opts(vars, data, opts) {
                    Ok(v) => Ok(v), // should: check type of e2?
                    Err(e) => match e.is_missing_data() {
                        true => e2.eval_in_row_opts(vars, data, opts),
                        false => Err(e),
                    },
                }
            }

            Self::FromUtf8(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::BinaryString(bs) => Ok(Value::UnicodeString(
                    String::from_utf8(bs)
                        .map_err(|e| EvalError::FromUtf8(e.to_string()))?,
                )),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for 'from_utf8' \
					 (expected: binary string)",
                )),
            },

            Self::FromUtf8Lossy(e) => {
                match e.eval_in_row_opts(vars, data, opts)? {
                    Value::BinaryString(bs) => Ok(Value::UnicodeString(
                        String::from_utf8_lossy(&bs).to_string(),
                    )),
                    _ => Err(EvalError::TypeError(
                        "invalid argument type for 'from_utf8_lossy' \
					 (expected: binary string)",
                    )),
                }
            }

            // Self::FromUtf16(e) => match e.eval_in_row_opts(vars, data, opts)? {
            //     Value::BinaryString(bs) => Ok(Value::UnicodeString(
            //         String::from_utf16(bs)
            //             .map_err(|e| EvalError::FromUtf16(e))?,
            //     )),
            //     _ => Err(EvalError::TypeError(
            //         "invalid argument type for 'from_utf16' \
            // 		 (expected: binary string)",
            //     )),
            // },

            // Self::FromUtf16Lossy(e) => {
            //     match e.eval_in_row_opts(vars, data, opts)? {
            //         Value::BinaryString(bs) => Ok(Value::UnicodeString(
            //             String::from_utf16_lossy(&bs).to_string(),
            //         )),
            //         _ => Err(EvalError::TypeError(
            //             "invalid argument type for 'from_utf16_lossy' \
            // 			 (expected: binary string)",
            //         )),
            //     }
            // }
            Self::ToBinary(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::UnicodeString(s) => {
                    Ok(Value::BinaryString(s.into_bytes()))
                }
                _ => Err(EvalError::TypeError(
                    "invalid argument type for 'to_binary' \
					 (expected: unicode string)",
                )),
            },

            Self::ParseInt(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::UnicodeString(v) => match v.parse() {
                    Ok(v) => Ok(Value::Integer(v)),
                    Err(_) => Err(EvalError::NumParseError(
                        "invalid input for parse_int",
                    )),
                },
                Value::BinaryString(v) => match &opts.types.strict_strings {
                    false => match String::from_utf8_lossy(&v).parse() {
                        Ok(v) => Ok(Value::Integer(v)),
                        Err(_) => Err(EvalError::NumParseError(
                            "invalid input for parse_int",
                        )),
                    },
                    true => Err(EvalError::TypeError(
                        "parse_int on binary string while implicit \
						 string conversion is disabled",
                    )),
                },
                _ => Err(EvalError::TypeError(
                    "invalid argument type for \
					       'parse_int' function",
                )),
            },

            Self::ParseFloat(e) => {
                match e.eval_in_row_opts(vars, data, opts)? {
                    Value::UnicodeString(v) => match v.parse() {
                        Ok(v) => Ok(Value::Float(v)),
                        Err(_) => Err(EvalError::NumParseError(
                            "invalid input for parse_float",
                        )),
                    },
                    Value::BinaryString(v) => {
                        match &opts.types.strict_strings {
                            false => {
                                match String::from_utf8_lossy(&v).parse() {
                                    Ok(v) => Ok(Value::Float(v)),
                                    Err(_) => Err(EvalError::NumParseError(
                                        "invalid input for parse_float",
                                    )),
                                }
                            }
                            true => Err(EvalError::TypeError(
                                "parse_float on binary string while implicit \
								 string conversion is disabled",
                            )),
                        }
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid argument type for \
					       'parse_float' function",
                    )),
                }
            }

            Self::ParseMacBin(e) => {
                match e.eval_in_row_opts(vars, data, opts)? {
                    Value::BinaryString(v) => Ok(Value::MacAddress(
                        v.as_slice().try_into().map_err(|_| {
                            EvalError::AddrParseError(
                                "invalid address length for \
						   parse_mac_bin",
                            )
                        })?,
                    )),
                    _ => Err(EvalError::TypeError(
                        "invalid argument type for \
					       'parse_mac_bin' function",
                    )),
                }
            }

            Self::ParseIpv4Bin(e) => {
                match e.eval_in_row_opts(vars, data, opts)? {
                    Value::BinaryString(v) => Ok(Value::Ipv4Address(
                        v.as_slice().try_into().map_err(|_| {
                            EvalError::AddrParseError(
                                "invalid address length for \
						   parse_ipv4_bin",
                            )
                        })?,
                    )),
                    _ => Err(EvalError::TypeError(
                        "invalid argument type for \
					       'parse_ipv4_bin' function",
                    )),
                }
            }

            Self::ParseIpv6Bin(e) => {
                match e.eval_in_row_opts(vars, data, opts)? {
                    Value::BinaryString(v) => match v.len() {
                        16 => Ok(Value::Ipv6Address([
                            (v[0] as u16) << 8 | v[1] as u16,
                            (v[2] as u16) << 8 | v[3] as u16,
                            (v[4] as u16) << 8 | v[5] as u16,
                            (v[6] as u16) << 8 | v[7] as u16,
                            (v[8] as u16) << 8 | v[9] as u16,
                            (v[10] as u16) << 8 | v[11] as u16,
                            (v[12] as u16) << 8 | v[13] as u16,
                            (v[14] as u16) << 8 | v[15] as u16,
                        ])),
                        _ => Err(EvalError::AddrParseError(
                            "invalid address length for \
						 parse_ipv6_bin",
                        )),
                    },
                    _ => Err(EvalError::TypeError(
                        "invalid argument type for \
					 'parse_ipv6_bin' function",
                    )),
                }
            }

            Self::AgeFromSeconds(e) => match e
                .eval_in_row_opts(vars, data, opts)?
            {
                Value::Integer(v) => Ok(Value::Age(i64_to_duration(v)?)),
                Value::Float(v) => Ok(Value::Age(f64_to_duration(v)?)),
                Value::Quantity(q) => Ok(Value::Age(f64_to_duration(
                    q.convert(&Unit::Time(TimeUnit::Second(FracPrefix::Nano)))?
                        .0,
                )?)),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for \
					 age_from_seconds",
                )),
            },

            Self::EnumValue(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::IntEnum(v) => Ok(Value::Integer(v.get_value_int())),
                Value::Enum(v) => Ok(Value::UnicodeString(v.deconstruct().1)),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for \
					 'enum_value' function",
                )),
            },

            Self::UnwrapError(e) => {
                match e.eval_in_row_opts(vars, data, opts)? {
                    Value::Result(v) => match v.deconstruct() {
                        (t, _, Ok(v)) => Ok(v.cast_to(t.as_ref())?),
                        (_, _, Err(v)) => match v {
                            Value::UnicodeString(v) => {
                                Err(EvalError::ErrorValue(v))
                            }
                            Value::Enum(v) => {
                                Err(EvalError::ErrorValue(v.deconstruct().1))
                            }
                            Value::IntEnum(v) => Err(EvalError::ErrorValue(
                                v.get_value_str().to_string(),
                            )),
                            _ => Err(EvalError::TypeError(
                                "invalid argument type for \
						       'unwrap_error' (expected: \
						       Result(_, String / Enum))",
                            )),
                        },
                    },
                    _ => Err(EvalError::TypeError(
                        "invalid argument type for 'unwrap_error' \
					       (expected: Result(_, String / Enum))",
                    )),
                }
            }

            Self::SubStr(e1, e2, e3) => match (
                e1.eval_in_row_opts(vars, data, opts)?,
                e2.eval_in_row_opts(vars, data, opts)?,
                e3.eval_in_row_opts(vars, data, opts)?,
            ) {
                (
                    Value::UnicodeString(v1),
                    Value::Integer(v2),
                    Value::Integer(v3),
                ) => match v1.get(v2 as usize..(v2 + v3) as usize) {
                    Some(v) => Ok(Value::UnicodeString(v.to_string())),
                    None => Err(EvalError::OutOfBounds),
                },
                (
                    Value::BinaryString(v1),
                    Value::Integer(v2),
                    Value::Integer(v3),
                ) => match v1.get(v2 as usize..(v2 + v3) as usize) {
                    Some(v) => Ok(Value::BinaryString(v.to_vec())),
                    None => Err(EvalError::OutOfBounds),
                },
                _ => Err(EvalError::TypeError(
                    "invalid argument types for 'substr' function \
					 (expected: unicode / binary string, int, int)",
                )),
            },

            Self::Concat(e1, e2) => {
                match (
                    e1.eval_in_row_opts(vars, data, opts)?,
                    e2.eval_in_row_opts(vars, data, opts)?,
                ) {
                    (Value::BinaryString(v1), Value::BinaryString(v2)) => {
                        let mut v = v1.clone();
                        v.extend(v2);
                        Ok(Value::BinaryString(v))
                    }
                    (
                        Value::UnicodeString(mut v1),
                        Value::UnicodeString(v2),
                    ) => {
                        v1.push_str(&v2);
                        Ok(Value::UnicodeString(v1))
                    }
                    _ => Err(EvalError::TypeError(
                        "invalid types for string concatenation",
                    )),
                }
            }

            Self::Format(f, e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::Integer(v) => Ok(Value::UnicodeString(
                    PythonFormat.format(f, &[v])?.to_string(),
                )),
                Value::Float(v) => Ok(Value::UnicodeString(
                    PythonFormat.format(f, &[v])?.to_string(),
                )),
                Value::Quantity(v) => Ok(Value::UnicodeString(
                    PythonFormat.format(f, &[v])?.to_string(),
                )),
                _ => Err(EvalError::TypeError("invalid types for format")),
            },

            Self::ToString(e) => Ok(Value::UnicodeString(
                e.eval_in_row_opts(vars, data, opts)?.into_string()?,
            )),

            Self::RegSubst(e, r, s) => {
                match e.eval_in_row_opts(vars, data, opts)? {
                    Value::UnicodeString(v) => Ok(Value::UnicodeString(
                        r.replace_all(v.as_str(), s.as_str()).to_string(),
                    )),
                    Value::BinaryString(v) => match &opts.types.strict_strings {
						false => Ok(Value::UnicodeString(
							r.replace_all(String::from_utf8_lossy(&v).as_ref(), s.as_str()).to_string(),
						)),
						true => Err(EvalError::TypeError(
							"regex substitution on binary string while implicit \
							 string conversion is disabled"
						))
					},
                    _ => Err(EvalError::TypeError(
                        "invalid type for regex substitution \
					       (expected: unicodestring)",
                    )),
                }
            }

            Self::HexStr(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::BinaryString(v) => Ok(Value::UnicodeString(
                    v.iter()
                        .map(|c| format!("{:02x}", c))
                        .collect::<Vec<_>>()
                        .join(":"),
                )),
                _ => Err(EvalError::TypeError("invalid type for hex_string")),
            },

            Self::SHA1(e) => match e.eval_in_row_opts(vars, data, opts)? {
                // Value::UnicodeString(v) => {
                //     Ok(Value::UnicodeString(format!("sha1:{:?}", v).into()))
                // } // TODO
                // Value::BinaryString(v) => {
                //     Ok(Value::UnicodeString(format!("sha1:{:?}", v).into()))
                // } // TODO
                _ => Err(EvalError::TypeError(
                    "the sha1 function is not yet implemented",
                )),
            },

            Self::MD5(e) => match e.eval_in_row_opts(vars, data, opts)? {
                // Value::UnicodeString(v) => {
                //     Ok(Value::UnicodeString(format!("md5:{:?}", v).into()))
                // } // TODO
                // Value::BinaryString(v) => {
                //     Ok(Value::UnicodeString(format!("md5:{:?}", v).into()))
                // } // TODO
                _ => Err(EvalError::TypeError(
                    "the md5 function is not yet implemented",
                )),
            },

            Self::NotEmpty(e) => match e.eval_in_row_opts(vars, data, opts)? {
                Value::UnicodeString(v) => match v.is_empty() {
                    false => Ok(Value::UnicodeString(v)),
                    true => Err(EvalError::InvalidValue),
                },
                Value::BinaryString(v) => match v.is_empty() {
                    false => Ok(Value::BinaryString(v)),
                    true => Err(EvalError::InvalidValue),
                },
                _ => Err(EvalError::TypeError("invalid type for not_empty")),
            },

            Self::UnpackTime(e) => {
                match e.eval_in_row_opts(vars, data, opts)? {
                    Value::BinaryString(v) => match v.as_slice() {
                        [year2, year1, month, day, hour, min, sec] => {
                            Ok(Value::Time(
                                Utc.from_utc_datetime(
                                    &NaiveDate::from_ymd_opt(
                                        *year1 as i32 | (*year2 as i32) << 8,
                                        *month as u32,
                                        *day as u32,
                                    )
                                    .ok_or(EvalError::ValueError(
                                        "invalid date",
                                    ))?
                                    .and_hms_opt(
                                        *hour as u32,
                                        *min as u32,
                                        *sec as u32,
                                    )
                                    .ok_or(EvalError::ValueError(
                                        "invalid time",
                                    ))?,
                                ),
                            ))
                        }
                        [year2, year1, month, day, hour, min, sec, centi] => {
                            Ok(Value::Time(
                                Utc.from_utc_datetime(
                                    &NaiveDate::from_ymd_opt(
                                        *year1 as i32 | (*year2 as i32) << 8,
                                        *month as u32,
                                        *day as u32,
                                    )
                                    .ok_or(EvalError::ValueError(
                                        "invalid date",
                                    ))?
                                    .and_hms_milli_opt(
                                        *hour as u32,
                                        *min as u32,
                                        *sec as u32,
                                        *centi as u32 * 10,
                                    )
                                    .ok_or(EvalError::ValueError(
                                        "invalid time",
                                    ))?,
                                ),
                            ))
                        }
                        _ => Err(EvalError::ValueError(
                            "invalid string for unpack_time",
                        )),
                    },
                    _ => Err(EvalError::TypeError(
                        "invalid type for unpack_time",
                    )),
                }
            }

            Self::Quantity(e, u) => match e
                .eval_in_row_opts(vars, data, opts)?
            {
                Value::Integer(v) => {
                    Ok(Value::Quantity(Quantity(v as f64, u.clone())))
                }
                Value::Float(v) => Ok(Value::Quantity(Quantity(v, u.clone()))),
                Value::Quantity(v) => Ok(Value::Quantity(v.convert(u)?)),
                _ => Err(EvalError::TypeError(
                    "invalid type for unit ascription",
                )),
            },

            Self::Convert(e, u) => {
                match e.eval_in_row_opts(vars, data, opts)? {
                    Value::Quantity(v) => Ok(Value::Quantity(v.convert(u)?)),
                    _ => Err(EvalError::TypeError(
                        "invalid type for unit conversion",
                    )),
                }
            }
        }
    }

    pub fn check_in_row<'a>(
        &self,
        vars: Option<&'a HashMap<&'a str, EvalCell<'a, Type, Type>>>,
        data: Option<&Type>,
    ) -> Result<Type, EvalError> {
        self.check_in_row_opts(vars, data, &EvalOpts::default())
    }

    pub fn check_in_row_opts<'a>(
        &self,
        vars: Option<&'a HashMap<&'a str, EvalCell<'a, Type, Type>>>,
        data: Option<&Type>,
        opts: &EvalOpts,
    ) -> Result<Type, EvalError> {
        match self {
            Self::Literal(v) => Ok(v.get_type()),

            Self::Data => match data {
                Some(t) => Ok(t.clone()),
                None => Err(EvalError::DataError(DataError::Missing)),
            },

            Self::Variable(n) => match vars.and_then(|v| v.get(n.as_str())) {
                Some(c) => c
                    .eval(|e, d| e.check_in_row_opts(vars, d, opts))
                    .map_err(|e| EvalError::VariableError(n.clone(), Box::new(e))),
                None => Err(EvalError::MissingVariable(n.clone())),
            },

            Self::Or(e1, e2) => {
                match (e1.check_in_row_opts(vars, data, opts)?, e2.check_in_row_opts(vars, data, opts)?) {
                    (Type::Boolean, Type::Boolean) => Ok(Type::Boolean),
                    _ => Err(EvalError::TypeError("invalid types for boolean or")),
                }
            }

            Self::And(e1, e2) => match (e1.check_in_row_opts(vars, data, opts)?, e2.check_in_row_opts(vars, data, opts)?)
            {
                (Type::Boolean, Type::Boolean) => Ok(Type::Boolean),
                _ => Err(EvalError::TypeError("invalid types for boolean and")),
            },

            Self::Not(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::Boolean => Ok(Type::Boolean),
                _ => Err(EvalError::TypeError("invalid types for boolean not")),
            },

            Self::Le(e1, e2)
				| Self::Lt(e1, e2)
				| Self::Gt(e1, e2)
				| Self::Ge(e1, e2)=> {
                match (e1.check_in_row_opts(vars, data, opts)?, e2.check_in_row_opts(vars, data, opts)?) {
                    (Type::BinaryString, Type::BinaryString) => Ok(Type::Boolean),
                    (Type::UnicodeString, Type::UnicodeString) => Ok(Type::Boolean),
                    (Type::Integer, Type::Integer) => Ok(Type::Boolean),
                    (Type::Float, Type::Float) => Ok(Type::Boolean),
					(Type::Time, Type::Time) => Ok(Type::Boolean),
					(Type::Age, Type::Age) => Ok(Type::Boolean),
                    _ => Err(EvalError::TypeError(
                        "invalid types for comparison operator",
                    )),
                }
            }

            Self::Eq(e1, e2)
				| Self::Ne(e1, e2) => {
                match (e1.check_in_row_opts(vars, data, opts)?, e2.check_in_row_opts(vars, data, opts)?) {
                    (Type::BinaryString, Type::BinaryString) => Ok(Type::Boolean),
                    (Type::UnicodeString, Type::UnicodeString) => Ok(Type::Boolean),
					(Type::Time, Type::Time) => Ok(Type::Boolean),
					(Type::Age, Type::Age) => Ok(Type::Boolean),
                    (Type::Integer, Type::Integer) => Ok(Type::Boolean),
                    (Type::Float, Type::Float) => Ok(Type::Boolean),
                    _ => Err(EvalError::TypeError(
                        "invalid types for comparison operator",
                    )),
                }
            }

            Self::Add(e1, e2) => match (
				e1.check_in_row_opts(vars, data, opts)?,
                e2.check_in_row_opts(vars, data, opts)?,
			) {
				(Type::Time, Type::Age) | (Type::Age, Type::Time) => Ok(Type::Time),
				(Type::Age, Type::Age) => Ok(Type::Age),
				(t1,t2) => match NumericTypePair::from(t1,t2) {
					Some(NumericTypePair::Integer) => Ok(Type::Integer),
					Some(NumericTypePair::Float) => Ok(Type::Float),
					Some(NumericTypePair::Quantity(d1, d2)) => Ok(Type::Quantity((d1 + d2)?)),
					None => Err(EvalError::TypeError("invalid types for addition")),
				}
			},

            Self::Sub(e1, e2) => match (
				e1.check_in_row_opts(vars, data, opts)?,
                e2.check_in_row_opts(vars, data, opts)?,
			) {
				(Type::Time, Type::Age) => Ok(Type::Time),
				(Type::Time, Type::Time) => Ok(Type::Age),
				(Type::Age,Type::Age) => Ok(Type::Age),
				(t1,t2) => match NumericTypePair::from(t1,t2) {
					Some(NumericTypePair::Integer) => Ok(Type::Integer),
					Some(NumericTypePair::Float) => Ok(Type::Float),
					Some(NumericTypePair::Quantity(d1, d2)) => Ok(Type::Quantity((d1 - d2)?)),
					None => Err(EvalError::TypeError("invalid types for subtraction")),
				}
			},

            Self::Mul(e1, e2) => match NumericTypePair::from(
                e1.check_in_row_opts(vars, data, opts)?,
                e2.check_in_row_opts(vars, data, opts)?,
            ) {
                Some(NumericTypePair::Integer) => Ok(Type::Integer),
                Some(NumericTypePair::Float) => Ok(Type::Float),
                Some(NumericTypePair::Quantity(d1, d2)) => Ok(Type::Quantity((d1 * d2)?)),
                None => Err(EvalError::TypeError("invalid types for multiplication")),
            },

            Self::Div(e1, e2) => match NumericTypePair::from(
                e1.check_in_row_opts(vars, data, opts)?,
                e2.check_in_row_opts(vars, data, opts)?,
            ) {
                Some(NumericTypePair::Integer) => Ok(Type::Float),
                Some(NumericTypePair::Float) => Ok(Type::Float),
                Some(NumericTypePair::Quantity(d1, d2)) => Ok(Type::Quantity((d1 / d2)?)),
                None => Err(EvalError::TypeError("invalid types for division")),
            },

            Self::Pow(e1, e2) => match e1.check_in_row_opts(vars, data, opts)? {
                Type::Integer | Type::Float => match e2.check_in_row_opts(vars, data, opts)? {
                    Type::Integer | Type::Float => Ok(Type::Float),
                    _ => Err(EvalError::TypeError("invalid types for power")),
                },
                Type::Quantity(d) => match e2.eval(None) {
                    Ok(Value::Integer(i)) => Ok(Type::Quantity(d.powi(i as i32)?)),
                    _ => Err(EvalError::TypeError(
                        "quantity power is only allowed with constant integer exponent",
                    )),
                },
                _ => Err(EvalError::TypeError("invalid types for power")),
            },

            Self::Neg(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::Integer => Ok(Type::Integer),
                Type::Float => Ok(Type::Float),
                Type::Quantity(d) => Ok(Type::Quantity(d)),
				Type::Age => Ok(Type::Age),
                _ => Err(EvalError::TypeError("invalid types for negation")),
            },

            Self::Log(e1, e2) => match NumericTypePair::from(
                e1.check_in_row_opts(vars, data, opts)?,
                e2.check_in_row_opts(vars, data, opts)?,
            ) {
                Some(NumericTypePair::Integer) => Ok(Type::Float),
                Some(NumericTypePair::Float) => Ok(Type::Float),
                _ => Err(EvalError::TypeError("invalid types for log")),
            },

            Self::Abs(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::Integer => Ok(Type::Integer),
                Type::Float => Ok(Type::Float),
                Type::Quantity(d) => Ok(Type::Quantity(d)),
                _ => Err(EvalError::TypeError("invalid types for abs")),
            },

            Self::Sign(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::Integer => Ok(Type::Integer),
                Type::Float => Ok(Type::Integer),
                Type::Quantity(_) => Ok(Type::Integer),
                _ => Err(EvalError::TypeError("invalid types for sign")),
            },

            Self::BitsLE(e1, e2, e3) => match (
                e1.check_in_row_opts(vars, data, opts)?,
                e2.check_in_row_opts(vars, data, opts)?,
                e3.check_in_row_opts(vars, data, opts)?,
            ) {
                (Type::BinaryString, Type::Integer, Type::Integer) => Ok(Type::Integer),
                _ => Err(EvalError::TypeError(
                    "invalid argument types for 'bits_le' function \
					       (expected: binarystring, int, int)",
                )),
            },

            Self::BitsBE(e1, e2, e3) => match (
                e1.check_in_row_opts(vars, data, opts)?,
                e2.check_in_row_opts(vars, data, opts)?,
                e3.check_in_row_opts(vars, data, opts)?,
            ) {
                (Type::BinaryString, Type::Integer, Type::Integer) => Ok(Type::Integer),
                _ => Err(EvalError::TypeError(
                    "invalid argument types for 'bits_be' function \
					       (expected: binarystring, int, int)",
                )),
            },

            Self::Fallback(e1, e2) => {
                let t1 = e1.check_in_row_opts(vars, data, opts)?;
                let t2 = e2.check_in_row_opts(vars, data, opts)?;
                match t1 == t2 {
                    true => Ok(t1),
                    false => match (t1,t2) {
						(Type::BinaryString | Type::UnicodeString, Type::BinaryString | Type::UnicodeString) => match &opts.types.strict_strings {
							false => Ok(Type::UnicodeString),
							true => Err(EvalError::TypeError("fallback between binary and unicode string \
															  while implicit casting is disabled"))
						},
						(t1,t2) => match NumericTypePair::from(t1, t2) {
							Some(NumericTypePair::Integer) => Ok(Type::Integer),
							Some(NumericTypePair::Float) => Ok(Type::Float),
							Some(NumericTypePair::Quantity(d1, d2)) => Ok(Type::Quantity((d1 + d2)?)),
							None => Err(EvalError::TypeError("incompatible types for fallback")),
						}
					},
                }
            }

            Self::SubStr(e1, e2, e3) => match (
                e1.check_in_row_opts(vars, data, opts)?,
                e2.check_in_row_opts(vars, data, opts)?,
                e3.check_in_row_opts(vars, data, opts)?,
            ) {
                (Type::UnicodeString, Type::Integer, Type::Integer) => Ok(Type::UnicodeString),
                (Type::BinaryString, Type::Integer, Type::Integer) => Ok(Type::BinaryString),
                _ => Err(EvalError::TypeError(
                    "invalid argument types for 'substr' function \
					 (expected: unicode / binary string, int, int)",
                )),
            },

            Self::Concat(e1, e2) => {
                match (e1.check_in_row_opts(vars, data, opts)?, e2.check_in_row_opts(vars, data, opts)?) {
                    (Type::UnicodeString, Type::UnicodeString) => Ok(Type::UnicodeString),
                    (Type::BinaryString, Type::BinaryString) => Ok(Type::BinaryString),
                    _ => Err(EvalError::TypeError("invalid types for concat")),
                }
            }

			Self::FromUtf8(e) => match e.check_in_row_opts(vars, data, opts)? {
				Type::BinaryString => Ok(Type::UnicodeString),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for 'from_utf8' \
					 (expected: binary string)",
                )),
			}

			Self::FromUtf8Lossy(e) => match e.check_in_row_opts(vars, data, opts)? {
				Type::BinaryString => Ok(Type::UnicodeString),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for 'from_utf8_lossy' \
					 (expected: binary string)",
                )),
			}

			// Self::FromUtf16(e) => match e.check_in_row_opts(vars, data, opts)? {
			// 	Type::BinaryString => Ok(Type::UnicodeString),
            //     _ => Err(EvalError::TypeError(
            //         "invalid argument type for 'from_utf16' \
			// 		 (expected: binary string)",
            //     )),
			// }

			// Self::FromUtf16Lossy(e) => match e.check_in_row_opts(vars, data, opts)? {
			// 	Type::BinaryString => Ok(Type::UnicodeString),
            //     _ => Err(EvalError::TypeError(
            //         "invalid argument type for 'from_utf16_lossy' \
			// 		 (expected: binary string)",
            //     )),
			// }

			Self::ToBinary(e) => match e.check_in_row_opts(vars, data, opts)? {
				Type::UnicodeString => Ok(Type::BinaryString),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for 'to_binary' \
					 (expected: unicode string)",
                )),
			}

            Self::ParseInt(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::UnicodeString => Ok(Type::Integer),
				Type::BinaryString => match &opts.types.strict_strings {
					false => Ok(Type::Integer),
					true => Err(EvalError::TypeError(
                        "parse_int on binary string while implicit \
						 string conversion is disabled",
                    )),
				},
                _ => Err(EvalError::TypeError(
                    "invalid argument type for \
					 'parse_int' function",
                )),
            },

            Self::ParseFloat(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::UnicodeString => Ok(Type::Float),
				Type::BinaryString => match &opts.types.strict_strings {
					false => Ok(Type::Float),
					true => Err(EvalError::TypeError(
                        "parse_float on binary string while implicit \
						 string conversion is disabled",
                    )),
				},
                _ => Err(EvalError::TypeError(
                    "invalid argument type for \
					 'parse_float' function",
                )),
            },
            Self::ParseMacBin(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::BinaryString => Ok(Type::MacAddress),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for \
					 'parse_mac_bin' function",
                )),
            },

            Self::ParseIpv4Bin(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::BinaryString => Ok(Type::Ipv4Address),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for \
					 'parse_ipv4_bin' function",
                )),
            },

            Self::ParseIpv6Bin(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::BinaryString => Ok(Type::Ipv6Address),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for \
					 'parse_ipv6_bin' function",
                )),
            },

			Self::AgeFromSeconds(e) => match e.check_in_row_opts(vars, data, opts)? {
				Type::Integer | Type::Float | Type::Quantity(Dimension::Time) => Ok(Type::Age),
				_ => Err(EvalError::TypeError("invalid argument type for \
											   'age_from_seconds' function"))
			}

            Self::EnumValue(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::IntEnum(_) => Ok(Type::Integer),
                Type::Enum(_) => Ok(Type::UnicodeString),
                _ => Err(EvalError::TypeError(
                    "invalid argument type for \
					 'enum_value' function",
                )),
            },

            Self::UnwrapError(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::Result(t, e) => match e.as_ref() {
                    Type::UnicodeString | Type::Enum(_) | Type::IntEnum(_) => Ok(t.as_ref().clone()),
                    _ => Err(EvalError::TypeError(
                        "invalid argument type for 'unwrap_error' \
						   (expected: Result(_, String / Enum))",
                    )),
                },
                _ => Err(EvalError::TypeError(
                    "invalid argument type for 'unwrap_error' \
					       (expected: Result(_, String / Enum))",
                )),
            },

            Self::Format(f, e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::Integer => {
                    PythonFormat.format(f, &[0i64])?;
                    Ok(Type::UnicodeString)
                }
                Type::Float => {
                    PythonFormat.format(f, &[0f64])?;
                    Ok(Type::UnicodeString)
                }
                Type::Quantity(_) => {
                    PythonFormat.format(f, &[Quantity::from_value(0f64)])?;
                    Ok(Type::UnicodeString)
                }
                _ => Err(EvalError::TypeError("invalid type for format")),
            },

			Self::ToString(e) => match e.check_in_row_opts(vars, data, opts)? {
				Type::UnicodeString |
				Type::BinaryString |
				Type::Integer |
				Type::Float |
				Type::Quantity(_) |
				Type::Enum(_) |
				Type::IntEnum(_) |
				Type::Boolean |
				Type::Time |
				Type::Age |
				Type::MacAddress |
				Type::Ipv4Address |
				Type::Ipv6Address => Ok(Type::UnicodeString),
				/* If types are added that cannot be converted
				 * to string, these should recurse. */
				Type::Option(_) |
				Type::Result(_, _) |
				Type::List(_) |
				Type::Set(_) |
				Type::Map(_,_) |
				Type::Tuple(_) |
				Type::Json => Ok(Type::UnicodeString)
			},

            Self::RegSubst(e, _, _) => match e.check_in_row_opts(vars, data, opts)? {
                Type::UnicodeString => Ok(Type::UnicodeString),
				Type::BinaryString => match &opts.types.strict_strings {
					false => Ok(Type::UnicodeString),
					true => Err(EvalError::TypeError(
						"regex substitution on binary string while implicit \
						 string conversion is disabled"
					))
				}
                _ => Err(EvalError::TypeError(
                    "invalid type for regex substitution \
					 (expected: unicodestring)",
                )),
            },

            Self::HexStr(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::BinaryString => Ok(Type::UnicodeString),
                _ => Err(EvalError::TypeError("invalid type for hex_string")),
            },

            Self::NotEmpty(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::UnicodeString => Ok(Type::UnicodeString),
                Type::BinaryString => Ok(Type::BinaryString),
                _ => Err(EvalError::TypeError("invalid type for not_empty")),
            },

            Self::MD5(e) => match e.check_in_row_opts(vars, data, opts)? {
                // Type::BinaryString => Ok(Type::UnicodeString),
                // Type::UnicodeString => Ok(Type::UnicodeString),
                _ => Err(EvalError::TypeError("the md5 function is not yet implemented")),
            },

            Self::SHA1(e) => match e.check_in_row_opts(vars, data, opts)? {
                // Type::BinaryString => Ok(Type::UnicodeString),
                // Type::UnicodeString => Ok(Type::UnicodeString),
                _ => Err(EvalError::TypeError("the sha1 function is not yet implemented")),
            },

            Self::UnpackTime(e) => match e.check_in_row_opts(vars, data, opts)? {
                Type::BinaryString => Ok(Type::Time),
                _ => Err(EvalError::TypeError("invalid type for unpack_time")),
            },

            Self::Quantity(e, u) => match e.check_in_row_opts(vars, data, opts)? {
                Type::Integer => Ok(Type::Quantity(u.dimension())),
                Type::Float => Ok(Type::Quantity(u.dimension())),
                Type::Quantity(d) => Ok(Type::Quantity((d + u.dimension())?)),
                _ => Err(EvalError::TypeError("invalid type for unit ascription")),
            },

            Self::Convert(e, u) => match e.check_in_row_opts(vars, data, opts)? {
                Type::Quantity(d) => Ok(Type::Quantity((d + u.dimension())?)),
                _ => Err(EvalError::TypeError("invalid type for unit conversion")),
            },
        }
    }

    /*pub fn to_rust_in_row<'a>(
        &self, f: &mut fmt::Formatter<'_>,
        vars: Option<&'a HashMap<&'a str,EvalCell<'a,Type,Value>>>,
        data: Option<&Data>) -> Result<Value,EvalError> {

        match self {

        }

    }*/

    pub fn py_repr(&self) -> PyRepr {
        PyRepr(self)
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Literal(v) => write!(f, "{}", v),
            Expr::Data => write!(f, "@"),
            Expr::Variable(n) => write!(f, "${{{}}}", n),
            Expr::Or(e1, e2) => write!(f, "({}) || ({})", e1, e2),
            Expr::And(e1, e2) => write!(f, "({}) && ({})", e1, e2),
            Expr::Not(e) => write!(f, "!({})", e),
            Expr::Lt(e1, e2) => write!(f, "({}) < ({})", e1, e2),
            Expr::Le(e1, e2) => write!(f, "({}) <= ({})", e1, e2),
            Expr::Eq(e1, e2) => write!(f, "({}) == ({})", e1, e2),
            Expr::Ne(e1, e2) => write!(f, "({}) != ({})", e1, e2),
            Expr::Ge(e1, e2) => write!(f, "({}) >= ({})", e1, e2),
            Expr::Gt(e1, e2) => write!(f, "({}) > ({})", e1, e2),
            Expr::Add(e1, e2) => write!(f, "({}) + ({})", e1, e2),
            Expr::Sub(e1, e2) => write!(f, "({}) - ({})", e1, e2),
            Expr::Mul(e1, e2) => write!(f, "({}) * ({})", e1, e2),
            Expr::Div(e1, e2) => write!(f, "({}) / ({})", e1, e2),
            Expr::Pow(e1, e2) => write!(f, "({}) ^ ({})", e1, e2),
            Expr::Neg(e) => write!(f, "-({})", e),
            Expr::Log(b, e) => write!(f, "log({},{})", b, e),
            Expr::Abs(e) => write!(f, "abs({})", e),
            Expr::Sign(e) => write!(f, "sign({})", e),
            Expr::BitsLE(e1, e2, e3) => {
                write!(f, "bits_le({}, {}, {})", e1, e2, e3)
            }
            Expr::BitsBE(e1, e2, e3) => {
                write!(f, "bits_be({}, {}, {})", e1, e2, e3)
            }
            Expr::Fallback(e1, e2) => write!(f, "fallback({}, {})", e1, e2),
            Expr::FromUtf8(e) => write!(f, "from_utf8({})", e),
            Expr::FromUtf8Lossy(e) => write!(f, "from_utf8_lossy({})", e),
            // Expr::FromUtf16(e) => write!(f, "from_utf16({})", e),
            // Expr::FromUtf16Lossy(e) => write!(f, "from_utf16_lossy({})", e),
            Expr::ToBinary(e) => write!(f, "to_binary({})", e),
            Expr::ParseInt(e) => write!(f, "parse_int({})", e),
            Expr::ParseFloat(e) => write!(f, "parse_float({})", e),
            Expr::ParseMacBin(e) => write!(f, "parse_mac_bin({})", e),
            Expr::ParseIpv4Bin(e) => write!(f, "parse_ipv4_bin({})", e),
            Expr::ParseIpv6Bin(e) => write!(f, "parse_ipv6_bin({})", e),
            Expr::AgeFromSeconds(e) => write!(f, "age_from_seconds({})", e),
            Expr::EnumValue(e) => write!(f, "enum_value({})", e),
            Expr::UnwrapError(e) => write!(f, "unwrap_error({})", e),
            Expr::SubStr(e1, e2, e3) => {
                write!(f, "substr({}, {}, {})", e1, e2, e3)
            }
            Expr::Concat(e1, e2) => write!(f, "({}) <> ({})", e1, e2),
            Expr::Format(s, e) => write!(f, "format(\"{}\", {})", s, e),
            Expr::ToString(e) => write!(f, "to_string({})", e),
            Expr::RegSubst(e, r, s) => write!(f, "({})~s/{}/{}/", e, r, s),
            Expr::SHA1(e) => write!(f, "sha1({})", e),
            Expr::MD5(e) => write!(f, "md5({})", e),
            Expr::NotEmpty(e) => write!(f, "not_empty({})", e),
            Expr::HexStr(e) => write!(f, "hex_string({})", e),
            Expr::UnpackTime(e) => write!(f, "unpack_time({})", e),
            Expr::Quantity(e, u) => write!(f, "({}) {}", e, u),
            Expr::Convert(e, u) => write!(f, "convert({},{})", e, u),
        }
    }
}

impl Display for PyRepr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            Expr::Data => write!(f, "Self()"),
            Expr::Literal(val) => write!(f, "Constant({})", val.py_repr()),
            Expr::Variable(name) => {
                write!(f, "Variable(name={})", PyUnicode(name))
            }
            Expr::Or(e1, e2) => {
                write!(f, "Or({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::And(e1, e2) => {
                write!(f, "And({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Not(e1) => {
                write!(f, "Not({})", PyRepr(e1))
            }
            Expr::Le(e1, e2) => {
                write!(f, "Le({}, {})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Lt(e1, e2) => {
                write!(f, "Lt({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Eq(e1, e2) => {
                write!(f, "Eq({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Ne(e1, e2) => {
                write!(f, "Ne({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Gt(e1, e2) => {
                write!(f, "Gt({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Ge(e1, e2) => {
                write!(f, "Ge({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Add(e1, e2) => {
                write!(f, "Add({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Sub(e1, e2) => {
                write!(f, "Sub({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Mul(e1, e2) => {
                write!(f, "Mul({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Div(e1, e2) => {
                write!(f, "Div({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Pow(e1, e2) => {
                write!(f, "Pow({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Neg(e1) => {
                write!(f, "Neg({})", PyRepr(e1))
            }
            Expr::Quantity(expr, unit) => {
                write!(
                    f,
                    "Quantity({},{})",
                    PyRepr(expr),
                    PyUnicode(&unit.to_string())
                )
            }
            Expr::Convert(expr, unit) => {
                write!(
                    f,
                    "Convert({},{})",
                    PyRepr(expr),
                    PyUnicode(&unit.to_string())
                )
            }
            Expr::Fallback(e1, e2) => {
                write!(f, "Fallback({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::FromUtf8(expr) => {
                write!(f, "FromUtf8({})", PyRepr(expr))
            }
            Expr::FromUtf8Lossy(expr) => {
                write!(f, "FromUtf8Lossy({})", PyRepr(expr))
            }
            Expr::ToBinary(expr) => {
                write!(f, "ToBinary({})", PyRepr(expr))
            }
            Expr::ParseInt(expr) => {
                write!(f, "ParseInt({})", PyRepr(expr))
            }
            Expr::ParseFloat(expr) => {
                write!(f, "ParseFloat({})", PyRepr(expr))
            }
            Expr::ParseMacBin(expr) => {
                write!(f, "ParseMacBin({})", PyRepr(expr))
            }
            Expr::ParseIpv4Bin(expr) => {
                write!(f, "ParseIpv4Bin({})", PyRepr(expr))
            }
            Expr::ParseIpv6Bin(expr) => {
                write!(f, "ParseIpv6Bin({})", PyRepr(expr))
            }
            Expr::AgeFromSeconds(expr) => {
                write!(f, "AgeFromSeconds({})", PyRepr(expr))
            }
            Expr::EnumValue(expr) => write!(f, "EnumValue({})", PyRepr(expr)),
            Expr::UnwrapError(expr) => {
                write!(f, "UnwrapError({})", PyRepr(expr))
            }
            Expr::Concat(e1, e2) => {
                write!(f, "Concat({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Format(fmt, expr) => {
                write!(f, "Format({},{})", PyUnicode(fmt), PyRepr(expr))
            }
            Expr::ToString(expr) => write!(f, "ToString({})", PyRepr(expr)),
            Expr::RegSubst(expr, regex, sub) => {
                write!(
                    f,
                    "RegSubst({},{},{})",
                    PyRepr(expr),
                    PyUnicode(regex.as_str()),
                    PyUnicode(sub)
                )
            }
            Expr::SubStr(e1, e2, e3) => write!(
                f,
                "SubStr({},{},{})",
                PyRepr(e1),
                PyRepr(e2),
                PyRepr(e3)
            ),
            Expr::HexStr(expr) => write!(f, "HexStr({})", PyRepr(expr)),
            Expr::SHA1(expr) => write!(f, "SHA1({})", PyRepr(expr)),
            Expr::MD5(expr) => write!(f, "MD5({})", PyRepr(expr)),
            Expr::NotEmpty(expr) => write!(f, "NotEmpty({})", PyRepr(expr)),
            Expr::Log(e1, e2) => {
                write!(f, "Log({},{})", PyRepr(e1), PyRepr(e2))
            }
            Expr::Sign(expr) => write!(f, "Sign({})", PyRepr(expr)),
            Expr::Abs(expr) => write!(f, "Abs({})", PyRepr(expr)),
            Expr::BitsLE(e1, e2, e3) => write!(
                f,
                "BitsLE({},{},{})",
                PyRepr(e1),
                PyRepr(e2),
                PyRepr(e3)
            ),
            Expr::BitsBE(e1, e2, e3) => write!(
                f,
                "BitsBE({},{},{})",
                PyRepr(e1),
                PyRepr(e2),
                PyRepr(e3)
            ),
            Expr::UnpackTime(expr) => {
                write!(f, "UnpackTime({})", PyRepr(expr))
            }
        }
    }
}

/* Constants for duration conversion. */
const SECONDS_RANGE: RangeInclusive<i64> = i64::MIN / 1000..=i64::MAX / 1000;
const MILLIS_RANGE: RangeInclusive<f64> =
    i64::MIN as f64 / 1e3..=i64::MAX as f64 / 1e3;
const MICROS_RANGE: RangeInclusive<f64> =
    i64::MIN as f64 / 1e6..=i64::MAX as f64 / 1e6;
const NANOS_RANGE: RangeInclusive<f64> =
    i64::MIN as f64 / 1e9..=i64::MAX as f64 / 1e9;

fn i64_to_duration(v: i64) -> Result<Duration, EvalError> {
    match SECONDS_RANGE.contains(&v) {
        true => Ok(Duration::seconds(v)),
        false => Err(EvalError::ValueError(
            "integer value too large for duration",
        )),
    }
}

fn f64_to_duration(v: f64) -> Result<Duration, EvalError> {
    match NANOS_RANGE.contains(&v) {
        true => Ok(Duration::nanoseconds(f64::round(v * 1e9) as i64)),
        false => match MICROS_RANGE.contains(&v) {
            true => Ok(Duration::microseconds(f64::round(v * 1e6) as i64)),
            false => match MILLIS_RANGE.contains(&v) {
                true => Ok(Duration::milliseconds(f64::round(v * 1e3) as i64)),
                false => Err(EvalError::ValueError(
                    "float value too large for duration",
                )),
            },
        },
    }
}
