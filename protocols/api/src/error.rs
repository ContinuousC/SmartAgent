/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use protocol::http;

use crate::input::PluginId;

pub type Result<T> = std::result::Result<T, Error>;
pub type TypeResult<T> = std::result::Result<T, TypeError>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Missing API plugin: {0}")]
    MissingPlugin(PluginId),
    #[error("Missing Required Configuration: {0}")]
    RequiredConfigError(String),

    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error("Unable to log in with the given credentials")]
    InvalidCredentials,
    #[error("Error during formating: {0}")]
    Format(#[from] std::fmt::Error),
    #[error("cannot resolve host: {0}")]
    ResolveHost(#[from] http::Error),

    #[error("VMWare: {0}")]
    VMWare(#[from] super::vmware::Error),
    #[error("MS Graph: {0}")]
    MSGraph(#[from] super::ms_graph::Error),
    #[error("Azure: {0}")]
    Azure(#[from] super::azure::Error),
    #[error("Ldap: {0}")]
    Ldap(#[from] super::ldap::Error),
    #[error("Cache: {0}")]
    Cache(#[from] super::cache::Error),
    #[error("Mirth: {0}")]
    Mirth(#[from] super::mirth::Error),
    #[error("Unity: {0}")]
    Unity(#[from] super::unity::Error),
    #[error("Xenapp: {0}")]
    XenApp(#[from] super::xenapp_director::Error),
    #[error("Proxmox: {0}")]
    Proxmox(#[from] super::proxmox::Error),
    #[error("Elastic: {0}")]
    Elastic(#[from] super::elastic::Error),

    #[error("{0}")]
    Custom(String),

    #[error("IO Error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Unable to (de)serialize: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("Error with the counters: {0}")]
    Counters(#[source] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum DTError {
    #[error("Command not found: {0}")]
    CommandNotFound(String),
    #[error("API Plugin Error: {0}")]
    Plugin(String),
    #[error("VMWare: {0}")]
    VMWare(#[from] super::vmware::DTError),
    #[error("MS Graph: {0}")]
    MSGraph(#[from] super::ms_graph::DTError),
    #[error("Azure: {0}")]
    Azure(#[from] super::azure::DTError),
    #[error("Ldap: {0}")]
    Ldap(#[from] super::ldap::Error),
    #[error("Cache: {0}")]
    Cache(#[from] super::cache::DTError),
    #[error("Mirth: {0}")]
    Mirth(#[from] super::mirth::DTError),
    #[error("Unity: {0}")]
    Unity(#[from] super::unity::DTError),
    #[error("Xenapp: {0}")]
    XenApp(#[from] super::xenapp_director::DTError),
    #[error("Proxmox: {0}")]
    Proxmox(#[from] super::proxmox::DTError),
    #[error("Elastic: {0}")]
    Elastic(#[from] super::elastic::DTError),
    //External(String, String)
}

#[derive(thiserror::Error, Debug)]
pub enum DTWarning {
    #[error("VMWare: {0}")]
    VMWare(#[from] super::vmware::DTWarning),
    #[error("MS Graph: {0}")]
    MSGraph(#[from] super::ms_graph::DTWarning),
    #[error("Azure: {0}")]
    Azure(#[from] super::azure::DTWarning),
    #[error("Ldap: {0}")]
    Ldap(#[from] super::ldap::Error),
    #[error("Cache: {0}")]
    Cache(#[from] super::cache::DTWarning),
    #[error("Mirth: {0}")]
    Mirth(#[from] super::mirth::DTWarning),
    #[error("Unity: {0}")]
    Unity(#[from] super::unity::DTWarning),
    #[error("Xenapp: {0}")]
    XenApp(#[from] super::xenapp_director::DTWarning),
    #[error("Proxmox: {0}")]
    Proxmox(#[from] super::proxmox::DTWarning),
    #[error("Elastic: {0}")]
    Elastic(#[from] super::elastic::DTWarning),
    //External(String, String)
}

#[derive(Debug)]
pub enum LivestatusError {
    NotOMD,
    ConnectionError(std::io::Error),
    WriteError(std::io::Error),
    ReadError(std::io::Error),
}

#[derive(thiserror::Error, Debug, Clone)]
pub enum TypeError {
    #[error("Unable to parse {0:?}: {1}")]
    ParseError(String, String),
    #[error("(0)")]
    Format(#[from] std::fmt::Error),
    #[error("Enum {0} with missing values")]
    EnumMissingVariables(String),
}
