/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use agent_utils::DBObj;
use agent_utils::TryGetFrom;

use super::error::TypeResult;
use super::index::Index;
use super::input::{Input, ObjectId};

#[derive(DBObj, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct EntrySpec {
    pub index: Vec<ObjectId>,
    pub implied_index: Option<ObjectId>,
    pub augments: Option<ObjectId>,
    pub fold: Option<u64>,
}

impl EntrySpec {
    pub fn get_index(&self, input: &Input) -> TypeResult<Index> {
        let mut index = match &self.augments {
            Some(object_id) => {
                object_id.try_get_from(&input.tables)?.get_index(input)?
            }
            None => Index::empty(),
        };

        for object_id in &self.index {
            index.vars.push(object_id.clone());
        }

        if let Some(implied) = &self.implied_index {
            index.implied = Some(implied.clone());
        }

        Ok(index)
    }
}
