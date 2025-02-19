/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{BTreeSet, HashSet};

use linked_hash_map::LinkedHashMap;
use linked_hash_set::LinkedHashSet;
use serde::{Deserialize, Serialize};

use etc_base::{DataFieldId, Row};
use value::HashableValue;

use super::error::{QueryCheckResult, QueryError, QueryResult, QueryTypeError};
use super::key_set::KeySet;
use super::query::{Query, QueryType};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JoinOperand {
    pub query: Box<Query>,
    pub join_type: JoinType,
    pub join_key: Vec<DataFieldId>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    #[serde(rename = "inner")]
    Inner,
    #[serde(rename = "outer")]
    Outer,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum Select {
    #[serde(rename = "first")]
    First,
    #[serde(rename = "last")]
    Last,
}

pub fn check(
    left: &JoinOperand,
    right: &JoinOperand,
    ltable: QueryType,
    rtable: QueryType,
) -> QueryCheckResult<QueryType> {
    /* Join keys should have the same length. */

    if left.join_key.len() != right.join_key.len() {
        return Err(QueryTypeError::JoinKeyLengthMismatch);
    }

    for (lid, rid) in left.join_key.iter().zip(&right.join_key) {
        /* Join key fields should be available. */
        let lkey = ltable
            .fields
            .get(lid)
            .ok_or_else(|| QueryTypeError::MissingKey(lid.clone()))?;
        let rkey = rtable
            .fields
            .get(rid)
            .ok_or_else(|| QueryTypeError::MissingKey(rid.clone()))?;

        /* Corresponding fields in join key should have the same type. */
        if lkey != rkey {
            return Err(QueryTypeError::JoinKeyTypeMismatch(
                lid.clone(),
                rid.clone(),
                lkey.clone(),
                rkey.clone(),
            ));
        }

        /* Join key fields should be hashable. */
        if !lkey.is_hashable() {
            return Err(QueryTypeError::UnhashableKey(
                lid.clone(),
                lkey.clone(),
            ));
        }
    }

    /* At least one of the join keys should be a primary key. */

    let lkeys: BTreeSet<DataFieldId> = left.join_key.iter().cloned().collect();
    let rkeys: BTreeSet<DataFieldId> = right.join_key.iter().cloned().collect();
    let lmiss: KeySet = ltable.keys.missing(&lkeys);
    let rmiss: KeySet = rtable.keys.missing(&rkeys);

    if !lmiss.is_empty() && !rmiss.is_empty() {
        return Err(QueryTypeError::NoPrimaryKey(lmiss, rmiss));
    }

    /* Calculate new key set. */

    Ok(QueryType {
        keys: KeySet(
            left.join_key
                .iter()
                .zip(&right.join_key)
                .map(|(lkey, rkey)| {
                    Ok(ltable
                        .keys
                        .find_key(lkey)?
                        .union(&rtable.keys.find_key(rkey)?)
                        .cloned()
                        .collect())
                })
                .chain(lmiss.0.into_iter().map(Ok))
                .chain(rmiss.0.into_iter().map(Ok))
                .collect::<QueryCheckResult<_>>()?,
        ),
        fields: ltable.fields.into_iter().chain(rtable.fields).collect(),
        singleton: ltable.singleton && rtable.singleton,
    })
}

pub fn run(
    left: &JoinOperand,
    right: &JoinOperand,
    left_data: Vec<Row>,
    right_data: Vec<Row>,
) -> QueryResult<Vec<Row>> {
    /* Index data by common keys. */

    let (mut left_index, left_unindexed) = index(&left.join_key, left_data)?;
    let (mut right_index, right_unindexed) =
        index(&right.join_key, right_data)?;

    /* Find final key list. */

    let result_keys: LinkedHashSet<Vec<HashableValue>> =
        match (left.join_type, right.join_type) {
            (JoinType::Outer, JoinType::Outer) => left_index
                .keys()
                .cloned()
                .collect::<HashSet<_>>()
                .union(&right_index.keys().cloned().collect())
                .cloned()
                .collect(),
            (JoinType::Inner, JoinType::Inner) => left_index
                .keys()
                .cloned()
                .collect::<HashSet<_>>()
                .intersection(&right_index.keys().cloned().collect())
                .cloned()
                .collect(),
            (JoinType::Outer, JoinType::Inner) => {
                left_index.keys().cloned().collect()
            }
            (JoinType::Inner, JoinType::Outer) => {
                right_index.keys().cloned().collect()
            }
        };

    /* Join results. */

    let mut result_data: Vec<Row> = result_keys
        .into_iter()
        .map(|key| {
            let left_rows = left_index.remove(&key).unwrap_or_else(|| {
                vec![key_to_row(&left.join_key, key.clone())]
            });
            let right_rows = right_index.remove(&key).unwrap_or_else(|| {
                vec![key_to_row(&right.join_key, key.clone())]
            });
            cross(left_rows, right_rows)
        })
        .collect::<QueryResult<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    if left.join_type == JoinType::Outer {
        result_data.extend(left_unindexed);
    }
    if right.join_type == JoinType::Outer {
        result_data.extend(right_unindexed);
    }

    Ok(result_data)
}

pub(super) fn index(
    key: &Vec<DataFieldId>,
    data: Vec<Row>,
) -> QueryResult<(LinkedHashMap<Vec<HashableValue>, Vec<Row>>, Vec<Row>)> {
    let mut index = LinkedHashMap::new();
    let mut unindexed = Vec::new();

    for row in data {
        match key_from_row(key, &row)? {
            Some(key) => index.entry(key).or_insert_with(Vec::new),
            None => &mut unindexed,
        }
        .push(row);
    }

    Ok((index, unindexed))
}

fn key_from_row(
    key: &Vec<DataFieldId>,
    row: &Row,
) -> QueryResult<Option<Vec<HashableValue>>> {
    let mut result = Vec::new();

    for elem in key {
        match row.get(elem) {
            Some(Ok(val)) => match HashableValue::from_value(val.clone()) {
                Some(val) => result.push(val),
                None => {
                    return Err(QueryTypeError::UnhashableKey(
                        elem.clone(),
                        val.get_type(),
                    )
                    .into())
                }
            },
            Some(Err(_)) => return Ok(None),
            None => return Err(QueryTypeError::MissingKey(elem.clone()).into()),
        }
    }

    Ok(Some(result))
}

fn key_to_row(key: &Vec<DataFieldId>, data: Vec<HashableValue>) -> Row {
    key.clone()
        .into_iter()
        .zip(data.iter().map(|k| Ok(HashableValue::to_value(k.clone()))))
        .collect()
}

fn cross(left: Vec<Row>, right: Vec<Row>) -> QueryResult<Vec<Row>> {
    if left.len() == 1 {
        Ok(right
            .into_iter()
            .map(|row| row.into_iter().chain(left[0].clone()).collect())
            .collect())
    } else if right.len() == 1 {
        Ok(left
            .into_iter()
            .map(|row| row.into_iter().chain(right[0].clone()).collect())
            .collect())
    } else {
        Err(QueryError::Cross)
    }
}
