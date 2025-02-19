/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::BTreeSet;
use std::fmt;
use std::iter::once;

use serde::{Deserialize, Serialize};

use super::error::{QueryCheckResult, QueryTypeError};
use etc_base::DataFieldId;

/// The primary key for a table.
/// A doubly-nested set is needed to account
/// for equivalent keys as a result of joins.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct KeySet(pub BTreeSet<BTreeSet<DataFieldId>>);

impl KeySet {
    pub fn from_simple(set: BTreeSet<DataFieldId>) -> Self {
        KeySet(set.into_iter().map(|k| once(k).collect()).collect())
    }

    /// Find keys in self not covered by "keys".
    pub fn missing(&self, keys: &BTreeSet<DataFieldId>) -> Self {
        KeySet(
            self.0
                .iter()
                .filter(|key| key.is_disjoint(keys))
                .cloned()
                .collect(),
        )
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn find_key(
        &self,
        needle: &DataFieldId,
    ) -> QueryCheckResult<BTreeSet<DataFieldId>> {
        self.0
            .iter()
            .find(|key| key.contains(needle))
            .cloned()
            .ok_or_else(|| QueryTypeError::MissingKey(needle.clone()))
    }
}

impl fmt::Display for KeySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .iter()
                .map(|ks| ks
                    .iter()
                    .map(|k| format!("{}", k))
                    .collect::<Vec<String>>()
                    .join(" / "))
                .collect::<Vec<String>>()
                .join(", ")
        )
    }
}
