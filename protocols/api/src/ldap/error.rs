/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::path::PathBuf;

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Error while creating connection: {0}")]
    Connection(#[source] ldap3::LdapError),
    #[error("Error during ldap bind: {0}")]
    LdapBind(#[source] ldap3::LdapError),
    #[error("Error during connection termincation: {0}")]
    LdapUnBind(#[source] ldap3::LdapError),
    #[error("Error during ldap search: {0}")]
    LdapSearch(#[source] ldap3::LdapError),
    #[error("Error during connection drive: {0}")]
    Drive(#[source] ldap3::LdapError),
    #[error("Error: {0}")]
    Ldap(#[source] ldap3::LdapError),
    #[error("{0}")]
    AgentUtils(#[from] agent_utils::Error),
    #[error(
        "The credential in the keyvault is invalid (no username/password)"
    )]
    InvalidCredential,
    #[error("Cannot parse {0} to {1}")]
    Parse(String, String),
    #[error("A timeout has occured")]
    Timeout(#[from] tokio::time::error::Elapsed),
    #[error("IO Error while trying to read file ({1}): {0}")]
    Io(#[source] std::io::Error, PathBuf),
    #[error("Tls Error: {0}")]
    Tls(#[from] native_tls::Error),
    #[error("{0}")]
    Custom(String),
    #[error("Service {0}: {1}")]
    Service(String, Box<Error>),
    #[error("Recieved ab unexpected replication status: {0}")]
    UnexpectedReplicationStatus(String),
    #[error("Recieved ab unexpected replication status ({0}): {1}")]
    UnexpectedReplicationStatusJSON(String, serde_json::Error),
    #[error("Attribute '{0}' not found in dn '{1}'")]
    AttributeNotFound(String, String),
}

impl Error {
    pub fn to_api(self) -> crate::Error {
        crate::Error::Ldap(self)
    }

    pub fn to_dtwarning(self) -> crate::DTWarning {
        crate::DTWarning::Ldap(self)
    }

    pub fn for_service(self, service: &str) -> Self {
        Error::Service(service.to_string(), Box::new(self))
    }
}
