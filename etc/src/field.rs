/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use agent_utils::DBObj;
use etc_base::Row;
use expression::{EvalCell, Expr};
use unit::{DecPrefix, DimensionlessUnit, Unit};
use value::{Data, DataError, Type, Value};

use crate::event_category::EventCategory;
use crate::source::Source2;

use super::source::Source;
use super::threshold::{threshold_compat, ThresholdSpec};

#[derive(Serialize, Deserialize, Clone, DBObj, Debug, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct FieldSpec {
    pub name: String,
    #[serde(default = "default_true")]
    pub monitoring: bool,
    #[serde(default = "default_false")]
    pub discovery: bool,
    /// Whether to include in check_mk output. Defaults to `monitoring` if unset.
    #[serde(rename = "Check_MK")]
    pub check_mk: Option<bool>,
    #[serde(default = "default_true")]
    pub elastic_data: bool,
    pub elastic_field: Option<String>,
    pub header: Option<String>,
    pub description: Option<String>,
    /// Added in EtcSyntax 0.98.35
    pub event_category: Option<EventCategory>,
    pub source: Source,
    /// Newer version of 'source', added in v1.07 / mnChecks_SmartM 0.99.42
    pub source2: Option<Source2>,
    pub input_type: Type,
    #[serde(default = "default_false")]
    pub inventorized: bool,
    #[serde(default = "default_false")]
    pub selector: bool,
    #[serde(default = "default_false")]
    pub perfdata: bool,
    #[serde(default, deserialize_with = "threshold_compat")]
    pub threshold: Option<ThresholdSpec>,
    pub numeric_format: Option<String>,
    pub display_unit: Option<Unit>,
    #[serde(default = "default_false")]
    pub auto_scale: bool,
    pub reference: Option<Expr>,
    pub relative_display_type: Option<RelativeDisplayType>,
    pub relative_format: Option<String>,
    pub time_display_type: Option<TimeDisplayType>,
    pub operators: Option<Vec<serde_json::Value>>,
    pub references: Option<HashMap<String, Expr>>,
    pub units: Option<Vec<Unit>>,
    #[serde(default)]
    pub expose_configrules: ExposeConfigRules,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimeDisplayType {
    Time,
    Age,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelativeDisplayType {
    Percentage,
    Ratio,
    Hidden,
}

#[derive(
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    smart_default::SmartDefault,
)]
#[serde(rename_all = "snake_case")]
pub enum ExposeConfigRules {
    #[default]
    User,
    MP,
    Both,
}

impl FieldSpec {
    const DEFAULT_EXPR: Expr = Expr::Data;

    pub fn field_expr<'a>(&'a self, row: &Row) -> EvalCell<'a, Data, Value> {
        match &self.source2 {
            Some(source2) => source2.field_expr(row),
            None => self.source.field_expr(row),
        }
    }

    pub fn event_category(&self) -> EventCategory {
        match self.event_category {
            Some(cat) => cat,
            None => match &self.input_type {
                Type::Integer | Type::Float | Type::Quantity(_) => {
                    EventCategory::Performance
                }
                _ => EventCategory::Availability,
            },
        }
    }
}

impl RelativeDisplayType {
    pub fn display_unit(&self) -> Unit {
        match self {
            RelativeDisplayType::Percentage => {
                Unit::Dimensionless(DimensionlessUnit::Percent)
            }
            _ => Unit::Dimensionless(DimensionlessUnit::Count(DecPrefix::Unit)),
        }
    }
}

impl Source2 {
    pub fn field_expr<'a>(&'a self, row: &Row) -> EvalCell<'a, Data, Value> {
        match self {
            Source2::Data(_data_table_id, data_field_id, expr) => {
                EvalCell::new(
                    expr.as_ref().unwrap_or(&FieldSpec::DEFAULT_EXPR),
                    Some(
                        row.get(data_field_id)
                            .cloned()
                            .unwrap_or(Err(DataError::Missing)),
                    ),
                )
            }
            Source2::Formula(expr) => EvalCell::new(expr, None),
            Source2::Config(expr) => EvalCell::new(
                expr.as_ref().unwrap_or(&FieldSpec::DEFAULT_EXPR),
                Some(Err(DataError::Missing)),
            ),
        }
    }
}

impl Source {
    pub fn field_expr<'a>(&'a self, row: &Row) -> EvalCell<'a, Data, Value> {
        match self {
            Source::Data(_data_table_id, data_field_id, expr) => EvalCell::new(
                expr.as_ref().unwrap_or(&FieldSpec::DEFAULT_EXPR),
                Some(
                    row.get(data_field_id)
                        .cloned()
                        .unwrap_or(Err(DataError::Missing)),
                ),
            ),
            Source::Formula(expr) => EvalCell::new(expr, None),
            Source::Config => EvalCell::new(
                &FieldSpec::DEFAULT_EXPR,
                Some(Err(DataError::Missing)),
            ),
        }
    }
}

const fn default_false() -> bool {
    false
}
const fn default_true() -> bool {
    true
}
