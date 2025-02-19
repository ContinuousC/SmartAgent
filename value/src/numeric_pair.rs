/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::{Type, Value};
use serde::{Deserialize, Serialize};
use unit::{Dimension, Quantity};

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum NumericTypePair {
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "float")]
    Float,
    #[serde(rename = "quantity")]
    Quantity(Dimension, Dimension),
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum NumericValuePair {
    #[serde(rename = "integer")]
    Integer(i64, i64),
    #[serde(rename = "float")]
    Float(f64, f64),
    #[serde(rename = "quantity")]
    Quantity(Quantity, Quantity),
}

impl NumericTypePair {
    pub fn from(left: Type, right: Type) -> Option<Self> {
        match (left, right) {
            (Type::Integer, Type::Integer) => Some(Self::Integer),
            (Type::Float | Type::Integer, Type::Float | Type::Integer) => {
                Some(Self::Float)
            }
            (Type::Quantity(d), Type::Integer | Type::Float)
            | (Type::Integer | Type::Float, Type::Quantity(d)) => {
                Some(Self::Quantity(d, Dimension::Dimensionless))
            }
            (Type::Quantity(l), Type::Quantity(r)) => {
                Some(Self::Quantity(l, r))
            }
            _ => None,
        }
    }
}

impl NumericValuePair {
    pub fn get_type(&self) -> NumericTypePair {
        match self {
            Self::Integer(_, _) => NumericTypePair::Integer,
            Self::Float(_, _) => NumericTypePair::Float,
            Self::Quantity(l, r) => {
                NumericTypePair::Quantity(l.dimension(), r.dimension())
            }
        }
    }

    pub fn from(left: Value, right: Value) -> Option<Self> {
        match (left, right) {
            (Value::Integer(l), Value::Integer(r)) => Some(Self::Integer(l, r)),
            (Value::Float(l), Value::Integer(r)) => {
                Some(Self::Float(l, r as f64))
            }
            (Value::Integer(l), Value::Float(r)) => {
                Some(Self::Float(l as f64, r))
            }
            (Value::Float(l), Value::Float(r)) => Some(Self::Float(l, r)),
            (Value::Quantity(l), Value::Integer(r)) => {
                Some(Self::Quantity(l, Quantity::from_value(r as f64)))
            }
            (Value::Quantity(l), Value::Float(r)) => {
                Some(Self::Quantity(l, Quantity::from_value(r)))
            }
            (Value::Integer(l), Value::Quantity(r)) => {
                Some(Self::Quantity(Quantity::from_value(l as f64), r))
            }
            (Value::Float(l), Value::Quantity(r)) => {
                Some(Self::Quantity(Quantity::from_value(l), r))
            }
            (Value::Quantity(l), Value::Quantity(r)) => {
                Some(Self::Quantity(l, r))
            }
            _ => None,
        }
    }
}
