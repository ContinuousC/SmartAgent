/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::any::Any;
use std::collections::HashMap;

use etc_base::{ProtoDataFieldId, ProtoDataTableId};

use super::data_field::DataFieldSpec;
use super::data_table::DataTableSpec;

#[derive(Debug)]
pub struct Input {
    pub handle: Box<dyn Any + Send + Sync>,
    pub data_tables: HashMap<ProtoDataTableId, DataTableSpec>,
    pub data_fields: HashMap<ProtoDataFieldId, DataFieldSpec>,
}
