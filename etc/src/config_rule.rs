/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use etc_base::FieldId;
use expression::EvalError;
use rule_engine::selector::ValueSelector;
use serde::{Deserialize, Serialize};
use value::Value;

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigRule {
    pub selectors: Vec<FieldSelector>,
    pub value: Value,
}

impl ConfigRule {
    pub fn evaled_matches(
        &self,
        row: &HashMap<FieldId, Result<value::Value, EvalError>>,
    ) -> Result<Option<Value>, EvalError> {
        let jvals = row
            .iter()
            .map(|(fid, val)| {
                Ok((fid, val.as_ref().map(|v| v.to_json_value())))
            })
            .collect::<Result<HashMap<_, _>, EvalError>>()?;

        let matches = self
            .selectors
            .iter()
            .map(|fsel| fsel.matches(&jvals))
            .collect::<Result<Vec<bool>, EvalError>>()?;

        if matches.into_iter().any(|b| b) {
            Ok(Some(self.value.clone()))
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct FieldSelector {
    field: FieldId,
    selector: ValueSelector,
}

impl FieldSelector {
    pub fn matches(
        &self,
        jvals: &HashMap<
            &FieldId,
            Result<Option<serde_json::Value>, &EvalError>,
        >,
    ) -> Result<bool, EvalError> {
        jvals
            .get(&self.field)
            .ok_or_else(|| EvalError::MissingVariable(self.field.to_string()))?
            .as_ref()
            .map_err(|e| (*e).clone())?
            .as_ref()
            .ok_or_else(|| {
                EvalError::ParseError(format!(
                    "Could not parse value of {} to json",
                    self.field
                ))
            })
            .map(|v| {
                self.selector
                    .matches(v, &None)
                    .map_err(|e| EvalError::Selector(e.to_string()))
            })?
    }
}
