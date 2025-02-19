/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::fmt;
use std::collections::HashMap;

use serde::{Serialize,Deserialize};
use schemars::JsonSchema;

use crate::error::Result;
use crate::specification::*;
use crate::database::{DBId,DBObj};
use crate::utils::{Key,TryAppend};


/* SSH-specific IDs. */

pub(super) type CommandId = DBId<CommandSpec>;

#[derive(Serialize,Deserialize,JsonSchema,Key,Hash,PartialEq,Eq,Clone,Debug)]
pub(super) struct ParserId(pub(super) String);



/* SSH Input specification. */

#[derive(DBObj,Serialize,Deserialize,JsonSchema,PartialEq,Eq,Debug)]
pub struct Input {
    #[serde(rename = "Commands")]
    pub(super) commands: HashMap<CommandId,CommandSpec>,
    #[serde(rename = "DataTables")]
    pub(super) data_tables: HashMap<DataTableId,CommandId>,
}


#[derive(DBObj,Serialize,Deserialize,JsonSchema,Debug,PartialEq,Eq)]
pub(super) struct CommandSpec {
    #[serde(rename = "CommandName")]
    pub(super) command_name: String,
    #[serde(rename = "CommandLine")]
    pub(super) command_line: String,
    #[serde(rename = "OutputType")]
    pub(super) parser: ParserId,
}


impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> Result<()> {
	self.commands.try_append(other.commands)?;
	self.data_tables.try_append(other.data_tables)?;
	Ok(())
    }
}

impl Default for Input {
    fn default() -> Self {
	Input {
	    commands: HashMap::new(),
	    data_tables: HashMap::new(),
	}
    }
}
