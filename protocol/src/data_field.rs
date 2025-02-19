/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use agent_utils::DBObj;
use value::Type;

#[derive(DBObj, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct DataFieldSpec {
    pub name: String,
    // Compat: this is now part of query
    //pub join_key: Option<JoinKey>,
    pub input_type: Type,
}
