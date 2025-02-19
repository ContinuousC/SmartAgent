/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use etc_base::{DataFieldId, Row};

use super::error::{QueryCheckResult, QueryResult, QueryTypeError};
use super::join;
use super::key_set::KeySet;
use super::query::QueryType;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Select {
    First,
    Last,
}

impl Select {
    fn select(&self, rows: Vec<Row>) -> Option<Row> {
        match self {
            Self::First => rows.into_iter().next(),
            Self::Last => rows.into_iter().last(),
        }
    }
}

pub(super) fn run(
    fields: &Vec<DataFieldId>,
    select: &Select,
    data: Vec<Row>,
) -> QueryResult<Vec<Row>> {
    let (index, _) = join::index(fields, data)?;
    Ok(index
        .into_iter()
        .filter_map(|(_, rows)| select.select(rows))
        .collect())
}

pub(super) fn check(
    fields: &Vec<DataFieldId>,
    table: QueryType,
) -> QueryCheckResult<QueryType> {
    for field_id in fields {
        match table.fields.get(field_id) {
            Some(typ) => {
                if !typ.is_hashable() {
                    return Err(QueryTypeError::UnhashableKey(
                        field_id.clone(),
                        typ.clone(),
                    ));
                }
            }
            None => {
                return Err(QueryTypeError::MissingKey(field_id.clone()));
            }
        }
    }

    Ok(QueryType {
        singleton: fields.is_empty(),
        fields: table.fields,
        keys: KeySet::from_simple(fields.iter().cloned().collect()),
    })
}
