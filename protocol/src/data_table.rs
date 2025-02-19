/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use agent_utils::DBObj;
use etc_base::ProtoDataFieldId;

#[derive(DBObj, Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct DataTableSpec {
    pub name: String,
    pub singleton: bool,
    pub keys: HashSet<ProtoDataFieldId>,
    pub fields: HashSet<ProtoDataFieldId>,
}
