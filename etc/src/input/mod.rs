/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod snmp;
mod wmi;
mod ssh;
mod api;
mod azure;


use serde::{Serialize,Deserialize};
use schemars::JsonSchema;

use crate::error::Result;
use crate::utils::TryAppend;
use super::{snmp,ssh,wmi,api,azure};


/// The input map contains protocol-specific information on
/// available parameters. The structure of the contents is
/// defined by the protocol plugins, which must define a
/// Deserializable type.

#[derive(Serialize,Deserialize,JsonSchema,PartialEq,Eq,Debug)]
pub struct Input {
    #[serde(rename="SNMP")]
    #[serde(default)]
    pub snmp: snmp::Input,
    #[serde(rename="SSH")]
    #[serde(default)]
    pub ssh: ssh::Input,
    #[serde(rename="WMI")]
    #[serde(default)]
    pub wmi: wmi::Input,
    #[serde(rename="API")]
    #[serde(default)]
    pub api: api::Input,
    #[serde(rename="Azure")]
    #[serde(default)]
    pub azure: azure::Input,
}


impl TryAppend for Input {
    fn try_append(&mut self, other: Self) -> Result<()> {
	self.snmp.try_append(other.snmp)?;
	self.wmi.try_append(other.wmi)?;
	self.ssh.try_append(other.ssh)?;
	self.azure.try_append(other.azure)?;
	Ok(())
    }
}

impl Default for Input {
    fn default() -> Self {
	Input {
	    snmp: snmp::Input::default(),
	    wmi: wmi::Input::default(),
	    ssh: ssh::Input::default(),
	    api: api::Input::default(),
	    azure: azure::Input::default(),
	}
    }
}
