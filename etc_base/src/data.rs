/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use super::ids::{DataFieldId, ProtoDataFieldId, ProtoDataTableId, Protocol};
use std::collections::{HashMap, HashSet};
use value::{Data, Type};

pub type QueryMap = HashMap<Protocol, ProtoQueryMap>;
pub type ProtoQueryMap = HashMap<ProtoDataTableId, HashSet<ProtoDataFieldId>>;

pub type Row = HashMap<DataFieldId, Data>;
pub type RowType = HashMap<DataFieldId, Type>;
pub type ProtoRow = HashMap<ProtoDataFieldId, Data>;
pub type ProtoJsonRow = HashMap<ProtoDataFieldId, ProtoJsonData>;
pub type ProtoRowType = HashMap<ProtoDataFieldId, Type>;
pub type ProtoJsonData = std::result::Result<serde_json::Value, String>;
