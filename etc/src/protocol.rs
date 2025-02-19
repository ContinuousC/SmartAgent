/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

/// Protocols are identified by a human-readable name
#[derive(Serialize,Deserialize,JsonSchema,Clone,Debug,Clone,Hash,PartialEq,Eq)]
pub enum Protocol {
    SNMP,
    SSH,
    WMI,
    API,
    Azure
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	match self {
	    Self::SNMP => write!(f, "SNMP"),
	    Self::SSH => write!(f, "SSH"),
	    Self::WMI => write!(f, "WMI"),
	    Self::API => write!(f, "API"),
	    Self::Azure => write!(f, "Azure"),
	}
    }
}
