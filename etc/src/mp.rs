/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use agent_utils::DBObj;
use etc_base::{CheckId, Tag};

#[derive(Serialize, Deserialize, Clone, DBObj, Debug, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct MPSpec {
    pub tag: Tag,
    pub name: String,
    pub description: Option<String>,
    pub checks: Option<Vec<CheckId>>,
}

impl MPSpec {
    pub fn elastic_name(&self) -> String {
        self.name
            .chars()
            .flat_map(|c| match c {
                'a'..='z' | '0'..='9' | '-' => vec![c],
                'A'..='Z' => c.to_lowercase().collect(),
                ' ' | '_' => vec!['-'],
                _ => vec![],
            })
            .collect()
    }
}
