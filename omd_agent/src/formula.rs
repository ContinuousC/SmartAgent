/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use expression::{EvalCell, EvalError, EvalResult, Expr};
use value::{Data, DataError, Value};

use etc::{FieldSpec, Source, Source2, TableSpec};
use etc_base::{FieldId, Row};

use crate::context::Context;
use crate::error::Result;
use agent_utils::TryGetFrom;

pub fn calculate_table(
    ctx: &Context,
    table: &TableSpec,
    data: Vec<Row>,
) -> Result<Vec<HashMap<FieldId, EvalResult>>> {
    let fields = table
        .fields
        .iter()
        .map(|field_id| {
            Ok((field_id, field_id.try_get_from(&ctx.spec.etc.fields)?))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(data
        .into_iter()
        .map(|row| calculate_row(&fields, row, ctx))
        .collect())
}

type EvaledRow = HashMap<FieldId, std::result::Result<value::Value, EvalError>>;

fn calculate_row(
    fields: &Vec<(&FieldId, &FieldSpec)>,
    row: Row,
    ctx: &Context,
) -> EvaledRow {
    let (conf_fields, expr_fields): (Vec<(_, _)>, Vec<(_, _)>) =
        fields.iter().partition(|(_fid, fspec)| {
            matches!(fspec.source2, Some(Source2::Config(_)))
        });

    let expr_row: HashMap<_, _> = expr_fields
        .into_iter()
        .map(|(_, field)| (field.name.as_str(), field_expr(field, &row)))
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
    let evaled_row: EvaledRow = fields
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
        .collect();

    let conf_row: HashMap<_, _> = conf_fields
        .into_iter()
        .map(|(fid, _fspec)| (fid.clone(), lookup_conf(fid, &evaled_row, ctx)))
        .collect();

    evaled_row.into_iter().chain(conf_row).collect()
}

const DATA_EXPR: Expr = Expr::Data;

fn field_expr<'a>(
    field: &'a FieldSpec,
    row: &Row,
) -> EvalCell<'a, Data, Value> {
    match &field.source {
        Source::Data(_data_table_id, data_field_id, expr) => EvalCell::new(
            expr.as_ref().unwrap_or(&DATA_EXPR),
            Some(
                row.get(data_field_id)
                    .cloned()
                    .unwrap_or(Err(DataError::Missing)),
            ),
        ),
        Source::Formula(expr) => EvalCell::new(expr, None),
        Source::Config => {
            EvalCell::new(&DATA_EXPR, Some(Err(DataError::Missing)))
        }
    }
}

fn lookup_conf(
    field_id: &FieldId,
    row: &EvaledRow,
    ctx: &Context,
) -> std::result::Result<Value, EvalError> {
    let mps = ctx.get_mps();

    ctx.spec
        .etc
        .config_rules
        .get(field_id)
        .ok_or(EvalError::MissingVariable(field_id.to_string()))?
        .iter()
        .filter_map(|(mpid, confrules)| mps.contains(mpid).then_some(confrules))
        .flatten()
        .filter_map(|conf_rule| conf_rule.evaled_matches(row).transpose())
        .next()
        .ok_or_else(|| EvalError::MissingVariable(field_id.to_string()))
        .and_then(std::convert::identity)
}

/*fn to_ipv6_address(bs: [u8;16]) -> [u16;8] {
    [(bs[0] as u16) << 8 | bs[1] as u16,
     (bs[2] as u16) << 8 | bs[3] as u16,
     (bs[4] as u16) << 8 | bs[5] as u16,
     (bs[6] as u16) << 8 | bs[7] as u16,
     (bs[8] as u16) << 8 | bs[9] as u16,
     (bs[10] as u16) << 8 | bs[11] as u16,
     (bs[12] as u16) << 8 | bs[13] as u16,
     (bs[14] as u16) << 8 | bs[15] as u16]
}
*/
