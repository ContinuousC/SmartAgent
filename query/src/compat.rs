/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use etc_base::{DataFieldId, DataTableId, JoinKey};

use super::error::{QueryCheckResult, QueryTypeError};
use super::join::{JoinOperand, JoinType};
use super::prefilter::PreFilter;
use super::query::{ErrorAction, Query};
use super::reindex::Select;

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub struct TableQuery {
    #[serde(rename = "DataTable")]
    pub data_table: DataTableId,
    #[serde(rename = "PreFilter")]
    pub pre_filter: PreFilter,
    #[serde(rename = "JoinType")]
    pub join_type: JoinType,
    #[serde(rename = "JoinKey")]
    pub join_key: HashMap<JoinKey, DataFieldId>,
    #[serde(rename = "ErrorAction")]
    pub error_action: Option<ErrorAction>,
    #[serde(rename = "IgnoreExistence")]
    pub ignore_existence: Option<bool>,
    #[serde(rename = "ReindexKeys")]
    pub reindex_keys: Option<Vec<DataFieldId>>,
    #[serde(rename = "ReindexSelect")]
    pub reindex_select: Option<Select>,
}

pub fn transform_table_queries(
    queries: Vec<TableQuery>,
) -> QueryCheckResult<Query> {
    match queries.is_empty() {
        true => Err(QueryTypeError::EmptyTableQuery),
        false => {
            let (query, _, _) = join_table_queries(queries);
            //debug!("Table query transformed to {:?}", &query);
            Ok(query)
        }
    }
}

fn join_table_queries(
    mut queries: Vec<TableQuery>,
) -> (Query, JoinType, HashMap<JoinKey, DataFieldId>) {
    let query = queries.pop().unwrap();
    let filtered = filter_table_query(&query);

    if queries.is_empty() {
        (filtered, query.join_type, query.join_key.clone())
    } else {
        let (left_query, left_join_type, left_join_keys) =
            join_table_queries(queries);

        let (left_join_key, right_join_key) = left_join_keys
            .iter()
            .filter_map(|(key, left)| {
                query
                    .join_key
                    .get(key)
                    .map(|right| (left.clone(), right.clone()))
            })
            .unzip();

        let result_query = Query::Join(
            JoinOperand {
                query: Box::new(left_query),
                join_type: left_join_type,
                join_key: left_join_key,
            },
            JoinOperand {
                query: Box::new(filtered),
                join_type: query.join_type,
                join_key: right_join_key,
            },
        );

        let result_join_type = match (left_join_type, query.join_type) {
            (JoinType::Inner, JoinType::Inner) => JoinType::Inner,
            _ => JoinType::Outer,
        };

        let result_join_keys = left_join_keys
            .into_iter()
            .chain(query.join_key.clone())
            .collect();

        (result_query, result_join_type, result_join_keys)
    }
}

fn filter_table_query(query: &TableQuery) -> Query {
    let data_query = Query::Data(
        query.data_table.clone(),
        query.error_action.clone().unwrap_or_default(),
        query.ignore_existence.unwrap_or(false),
    );

    let has_filter = match &query.pre_filter {
        PreFilter::All(cs) => !cs.is_empty(),
        PreFilter::NotIn { field: _, values } => !values.is_empty(),
        _ => true,
    };

    let filter_query = match has_filter {
        true => Query::Filter(query.pre_filter.clone(), Box::new(data_query)),
        false => data_query,
    };

    match &query.reindex_select {
        Some(select) => Query::Reindex(
            query.reindex_keys.clone().unwrap_or_default(),
            select.clone(),
            Box::new(filter_query),
        ),
        None => filter_query,
    }
}
