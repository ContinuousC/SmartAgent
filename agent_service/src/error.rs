/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;
use thiserror::Error;

/// Result type for use everywhere in the super agent
pub type Result<T> = std::result::Result<T, Error>;

/// Any agent error
#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("ETC error: {0}")]
    Utils(#[from] agent_utils::Error),
    #[error("Missing environment variable \"{1}\" in file \"{0}\"")]
    MissingEnvVarInFile(PathBuf, String),
    #[error("Syscall error: {0}")]
    Nix(#[from] nix::Error),
    #[error("Rtnetlink error: {0}")]
    RtNetlink(#[from] rtnetlink::Error),
    #[error("DNS resolution error: {0}")]
    Resolve(#[from] trust_dns_resolver::error::ResolveError),
    #[error("Nmap error: {0}")]
    Nmap(#[from] nmap::error::Error),
    #[error("SNMP error: {0}")]
    SnmpProto(#[from] snmp_protocol::Error),
    #[error("VMWare error: {0}")]
    VmWareProto(#[from] api_protocol::vmware::Error),
    #[error("Missing protocol config for {0}")]
    MissingProtoConfig(etc_base::Protocol),
    #[error("Missing ElasticIndex for {0}")]
    MissingElasticIndex(String),
    #[error("Missing ElasticField for {0}")]
    MissingElasticField(String),
    #[error("Scheduler error: {0}")]
    Scheduler(#[from] scheduler::Error),
    #[error("Protocol error: {0}")]
    Protocol(#[from] protocol::Error),
    #[error("Etc error: {0}")]
    Etc(#[from] etc::Error),
}
