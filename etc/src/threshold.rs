/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::convert::{TryFrom, TryInto};

use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;

use expression::Expr;
use value::Value;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum ThresholdLevel {
    Warning,
    Critical,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThresholdSpec {
    Selector {
        warning: Option<serde_json::Value>,
        critical: Option<serde_json::Value>,
    },
}

/* Backward compatibility. */

pub fn threshold_compat<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ThresholdSpec>, D::Error> {
    match <Option<ThresholdCompat>>::deserialize(deserializer)?
        .map(|thr| thr.try_into())
        .transpose()
    {
        Ok(val) => Ok(val),
        Err(e) => {
            log::warn!("unsupported threshold default: {}", e);
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(untagged)]
enum ThresholdCompat {
    ThresholdSpec(ThresholdSpec),
    ThresholdMap(ThresholdMap),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
struct ThresholdMap {
    warning: Option<ThresholdSpecV0>,
    critical: Option<ThresholdSpecV0>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ThresholdSpecV0 {
    Selector(Expr),
}

impl TryFrom<ThresholdCompat> for ThresholdSpec {
    type Error = ConversionError;
    fn try_from(
        value: ThresholdCompat,
    ) -> Result<ThresholdSpec, ConversionError> {
        Ok(match value {
            ThresholdCompat::ThresholdSpec(spec) => spec,
            ThresholdCompat::ThresholdMap(map) => ThresholdSpec::Selector {
                warning: map
                    .warning
                    .map(|ThresholdSpecV0::Selector(expr)| {
                        expr_to_selector_value(expr)
                    })
                    .transpose()?,
                critical: map
                    .critical
                    .map(|ThresholdSpecV0::Selector(expr)| {
                        expr_to_selector_value(expr)
                    })
                    .transpose()?,
            },
        })
    }
}

fn expr_to_selector_value(
    expr: Expr,
) -> Result<serde_json::Value, ConversionError> {
    match &expr {
        Expr::Gt(a, b) if a.as_ref() == &Expr::Data => {
            Ok(numeric_comparison_selector("gt", b.as_ref())
                .ok_or_else(|| unsupported_conversion(expr))?)
        }
        Expr::Ge(a, b) if a.as_ref() == &Expr::Data => {
            Ok(numeric_comparison_selector("ge", b.as_ref())
                .ok_or_else(|| unsupported_conversion(expr))?)
        }
        Expr::Le(a, b) if a.as_ref() == &Expr::Data => {
            Ok(numeric_comparison_selector("le", b.as_ref())
                .ok_or_else(|| unsupported_conversion(expr))?)
        }
        Expr::Eq(a, b) if a.as_ref() == &Expr::Data => {
            Ok(numeric_comparison_selector("eq", b.as_ref())
                .ok_or_else(|| unsupported_conversion(expr))?)
        }
        Expr::Ne(a, b) if a.as_ref() == &Expr::Data => {
            Ok(numeric_comparison_selector("ne", b.as_ref())
                .ok_or_else(|| unsupported_conversion(expr))?)
        }
        Expr::Le(a, b) if a.as_ref() == &Expr::Data => {
            Ok(numeric_comparison_selector("le", b.as_ref())
                .ok_or_else(|| unsupported_conversion(expr))?)
        }
        Expr::Lt(a, b) if a.as_ref() == &Expr::Data => {
            Ok(numeric_comparison_selector("lt", b.as_ref())
                .ok_or_else(|| unsupported_conversion(expr))?)
        }
        _ => Err(unsupported_conversion(expr)),
    }
}

fn numeric_comparison_selector(
    op: &'static str,
    expr: &Expr,
) -> Option<serde_json::Value> {
    match expr {
        Expr::Literal(Value::UnicodeString(s)) => match s.ends_with('%') {
            false => {
                let v: f64 = s.parse().ok()?;
                Some(json!({ "absolute": { op: v } }))
            }
            true => {
                let v: f64 = s[..s.len() - 1].parse().ok()?;
                Some(json!({ "relative": { op: v } }))
            }
        },
        //Expr::Variable(name) => todo!(),
        _ => None,
    }
}

fn unsupported_conversion(expr: Expr) -> ConversionError {
    match serde_json::to_value(expr.clone()) {
        Ok(v) => ConversionError::Unsupported(v),
        Err(e) => ConversionError::Json(e),
    }
}

#[derive(Error, Debug)]
pub enum ConversionError {
    #[error("Unsupported threshold: {}", serde_json::to_string(.0)
	    .ok().as_deref().unwrap_or("(encoding error)"))]
    Unsupported(serde_json::Value),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
