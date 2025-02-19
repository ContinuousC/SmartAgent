/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use jsonpath::Selector;
use serde::Deserialize;
use serde_json::Value;

use etc_base::{ProtoDataFieldId, ProtoRow};
use value::DataError;

use crate::input::FieldSpec;
use crate::ms_graph::parsers::parse_jsonval;

#[derive(Deserialize)]
pub struct Response {
    // id: Option<String>,
    value: Vec<Value>,
    // next_link: Option<String>
}

impl Response {
    pub fn to_datatable(
        &self,
        fields: &HashMap<ProtoDataFieldId, (FieldSpec, Selector)>,
    ) -> Vec<ProtoRow> {
        self.value
            .iter()
            .map(|val| {
                fields
                    .iter()
                    .map(|(df_id, (field, selector))| {
                        (
                            df_id.clone(),
                            if let Some(val) = selector.find(val).next() {
                                parse_jsonval(field, val.clone()).map_err(|e| {
                                    DataError::TypeError(e.to_string())
                                })
                            } else {
                                Err(DataError::Missing)
                            },
                        )
                    })
                    .collect::<ProtoRow>()
            })
            .collect::<Vec<ProtoRow>>()
    }
}
