/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use agent_utils::{DBObj, TryGetFrom};
use etc_base::{Annotated, FieldId, QueryId, Row};
use expression::{EvalError, EvalResult, Expr};
use protocol::DataMap;
use query::AnnotatedQueryResult;
use value::{DataError, Value};

use super::error::Result;
use super::etc::Etc;
use super::field::FieldSpec;
use super::layer::Layer;
use super::query_mode::QueryMode;

#[derive(Serialize, Deserialize, Clone, DBObj, Debug, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct TableSpec {
    pub query: QueryId,
    pub name: Option<String>,
    pub title: Option<String>,
    #[serde(default = "default_true")]
    pub monitoring: bool,
    #[serde(default = "default_false")]
    pub discovery: bool,
    pub layer: Option<Layer>,
    /// Whether to include in check_mk output. Defaults to `monitoring` if unset.
    #[serde(rename = "Check_MK")]
    pub check_mk: Option<bool>,
    pub elastic_index: Option<String>,
    pub description: Option<String>,
    #[serde(default = "default_false")]
    pub singleton: bool,
    pub item_type: Option<(String, String)>,
    pub item_id: Option<Expr>,
    pub item_name: Option<Expr>,
    pub sub_tables: Option<Vec<SubTableSpec>>,
    pub fields: Vec<FieldId>,
}

#[derive(Serialize, Deserialize, Clone, DBObj, Debug, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct SubTableSpec {
    pub name: Option<String>,
    pub fields: Vec<FieldId>,
}

impl TableSpec {
    pub fn calculate(
        &self,
        query_mode: QueryMode,
        etc: &Etc,
        data: &DataMap,
    ) -> Result<AnnotatedQueryResult<Vec<HashMap<FieldId, EvalResult>>>> {
        let query = self.query.try_get_from(&etc.queries)?;
        Ok(match query.run(data) {
            Ok(Annotated {
                value: rows,
                warnings,
            }) => Ok(Annotated {
                value: self.eval_exprs(query_mode, etc, rows)?,
                warnings,
            }),
            Err(e) => Err(e),
        })
    }

    fn eval_exprs(
        &self,
        query_mode: QueryMode,
        etc: &Etc,
        data: Vec<Row>,
    ) -> Result<Vec<HashMap<FieldId, EvalResult>>> {
        let fields = self.fields_for_mode(query_mode, etc)?;
        Ok(data
            .into_iter()
            .map(|row| calculate_row(&fields, row))
            .collect())
    }

    fn get_fields<'a>(
        &'a self,
        etc: &'a Etc,
    ) -> Result<Vec<(&'a FieldId, &'a FieldSpec)>> {
        self.fields
            .iter()
            .map(|field_id| Ok((field_id, field_id.try_get_from(&etc.fields)?)))
            .collect::<Result<_>>()
    }

    pub fn fields_for_mode<'a>(
        &'a self,
        query_mode: QueryMode,
        etc: &'a Etc,
    ) -> Result<Vec<(&'a FieldId, &'a FieldSpec)>> {
        match query_mode {
            QueryMode::CheckMk => self.checkmk_fields(etc),
            QueryMode::Monitoring => self.monitoring_fields(etc),
            QueryMode::Discovery => self.discovery_fields(etc),
        }
    }

    pub fn discovery_fields<'a>(
        &'a self,
        etc: &'a Etc,
    ) -> Result<Vec<(&'a FieldId, &'a FieldSpec)>> {
        Ok(self
            .get_fields(etc)?
            .into_iter()
            .filter(|(_field_id, field)| field.discovery)
            .collect())
    }

    pub fn monitoring_fields<'a>(
        &'a self,
        etc: &'a Etc,
    ) -> Result<Vec<(&'a FieldId, &'a FieldSpec)>> {
        Ok(self
            .get_fields(etc)?
            .into_iter()
            .filter(|(_field_id, field)| field.monitoring)
            .collect())
    }

    pub fn checkmk_fields<'a>(
        &'a self,
        etc: &'a Etc,
    ) -> Result<Vec<(&'a FieldId, &'a FieldSpec)>> {
        Ok(self
            .get_fields(etc)?
            .into_iter()
            .filter(|(_field_id, field)| {
                field.check_mk.unwrap_or(field.monitoring)
            })
            .collect())
    }
}

fn calculate_row(
    fields: &Vec<(&FieldId, &FieldSpec)>,
    row: Row,
) -> HashMap<FieldId, std::result::Result<Value, EvalError>> {
    let expr_row: HashMap<_, _> = fields
        .iter()
        .map(|(_, field)| (field.name.as_str(), field.field_expr(&row)))
        .collect();

    let mut eval_row: HashMap<_, _> = expr_row
        .iter()
        .map(|(field_name, cell)| {
            (
                field_name,
                cell.eval(|expr, data| expr.eval_in_row(Some(&expr_row), data)),
            )
        })
        .collect();

    fields
        .iter()
        .map(|(field_id, field)| {
            (
                (*field_id).clone(),
                eval_row
                    .remove(&field.name.as_str())
                    .unwrap_or(Err(EvalError::DataError(DataError::Missing)))
                    .and_then(|v| Ok(v.cast_to(&field.input_type)?)),
            )
        })
        .collect()
}

const fn default_true() -> bool {
    true
}
const fn default_false() -> bool {
    false
}

impl TableSpec {
    pub fn query_for(&self, mode: QueryMode) -> bool {
        match mode {
            QueryMode::Discovery => self.discovery,
            QueryMode::Monitoring => self.monitoring,
            QueryMode::CheckMk => self.check_mk.unwrap_or(self.monitoring),
        }
    }
}
