/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::fmt;

use netsnmp::Oid;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;
pub type TypeResult<T> = std::result::Result<T, TypeError>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ETC error: {0}")]
    Utils(#[from] agent_utils::Error),
    #[error("Authentication error: {0}")]
    Authentication(netsnmp::Error),
    #[error("Failed to connect: {0}")]
    Connection(netsnmp::Error),
    #[error("Query failed: {0}")]
    Query(netsnmp::Error),
    #[error("SNMP bulk optimization yielded empty query!")]
    EmptyQuery,
    #[error("Empty response!")]
    EmptyResponse,
    #[error("SNMP bulk optimization yielded invalid query!")]
    InvalidQuery,
    #[error("OID {0} was not requested (SNMP plugin error!)")]
    NotRequested(Oid),
    #[error("Incomplete parse on walk at line {0}: {1}")]
    IncompleteParseStored(u32, String),
    #[error("Parse error on stored walk:\n{0}")]
    StoredParseError(String),
    #[error("Incomplete input for stored walk!?")]
    IncompleteStoredWalk,
    #[error("SNMP from stored walk is not implemented!")]
    StoredWalkNotImplemented,
    #[error("Non-bulk SNMP is not implemented!")]
    NonBulkSNMPNotImplemented,
    #[error("Type error: {0}")]
    Type(#[from] TypeError),
    #[error("Failed to get next session!")]
    MissingSession,
    #[error("No IP found")]
    NoIP,
    #[error("Failed to lookup IP {0}!")]
    DNS(#[from] trust_dns_resolver::error::ResolveError),
}

#[derive(Error, Debug)]
pub enum TypeError {
    #[error("expected integer ValueMap")]
    ExpectedIntegerValueMap,
    #[error("expected string ValueMap")]
    ExpectedStringValueMap,
    #[error("expected integer ValueMap for BitStr")]
    ExpectedIntegerValueMapForBitStr,
    #[error("unimplemented SNMP type: {0:?}")]
    UnimplementedSnmpType(netsnmp::VarType),
    #[error("invalid data table id: {0}: not an entry object")]
    InvalidTableId(etc_base::ProtoDataTableId),
    #[error("invalid data field id: {0}: not a scalar object")]
    InvalidFieldId(etc_base::ProtoDataFieldId),
    #[error("invalid index field: {0}")]
    InvalidIndexField(super::input::ObjectId),
    #[error("not a field id: {0}")]
    InvalidField(super::input::ObjectId),
    #[error("not a table id: {0}")]
    InvalidTable(super::input::ObjectId),
    #[error("ETC error: {0}")]
    Utils(#[from] agent_utils::Error),
}

#[derive(Error, Clone, Debug)]
pub enum DTError {
    WalkErrs(HashMap<WalkError, HashSet<Oid>>),
}

#[derive(Error, Clone, Debug)]
pub enum DTWarning {
    WalkErrs(
        HashMap<WalkError, HashSet<Oid>>,
        HashMap<WalkWarning, HashSet<Oid>>,
    ),
}

#[derive(Error, PartialEq, Eq, Hash, Clone, Debug)]
pub enum WalkError {
    #[error("oids not increasing")]
    OIDsNotIncreasing,
    #[error("no such object")]
    NoSuchObject,
    #[error("timeout")]
    Timeout,
}

#[derive(Error, PartialEq, Eq, Hash, Clone, Debug)]
pub enum WalkWarning {
    #[error("oids not increasing")]
    OIDsNotIncreasing,
    #[error("timeout")]
    Timeout,
}

impl fmt::Display for DTError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WalkErrs(errs) => write!(
                f,
                "received {}",
                errs.iter()
                    .map(|(err, oids)| {
                        let mut sorted_oids: Vec<&Oid> = oids.iter().collect();
                        sorted_oids.sort();
                        format!(
                            "\"{}\" for oids ({})",
                            err,
                            sorted_oids
                                .iter()
                                .map(|oid| oid.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl fmt::Display for DTWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WalkErrs(errs, warns) => write!(
                f,
                "{}",
                errs.iter()
                    .map(|(err, oids)| {
                        let mut sorted_oids: Vec<&Oid> = oids.iter().collect();
                        sorted_oids.sort();
                        format!(
                            "{} ({})",
                            err,
                            sorted_oids
                                .iter()
                                .map(|oid| oid.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    })
                    .chain(warns.iter().map(|(warn, oids)| {
                        let mut sorted_oids: Vec<&Oid> = oids.iter().collect();
                        sorted_oids.sort();
                        format!(
                            "{} ({})",
                            warn,
                            sorted_oids
                                .iter()
                                .map(|oid| oid.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}
