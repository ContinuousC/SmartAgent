/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum EnumType {
    Integer(
        #[serde(with = "agent_serde::arc_intkey_map")]
        Arc<BTreeMap<i64, String>>,
    ),
    String(Arc<BTreeSet<String>>),
}
