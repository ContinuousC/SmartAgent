/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use etc_base::{ProtoDataFieldId, ProtoDataTableId};

use crate::soap::SoapClient;

#[derive(Debug)]
pub struct Command {
    pub soapclient: Arc<SoapClient>,
    pub args: HashMap<String, String>,
    pub table: ProtoDataTableId,
    pub fields: HashMap<ProtoDataFieldId, String>,
    pub ts_file: PathBuf,
    pub hostsystems: HashMap<String, String>,
    pub command_name: String,
    pub command_line: String,
}
