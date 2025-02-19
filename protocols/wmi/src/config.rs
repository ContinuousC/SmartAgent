/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use powershell_protocol as ps;

use serde::{Deserialize, Serialize};

use agent_utils::KeyVault;

use crate::dcom;
use crate::error::DTResult;
use crate::error::WMIDTError;
use crate::Result;
use crate::WMIError;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)]
    powershell: Option<ps::ConnectionConfig>,
    #[serde(default)]
    dcom: Option<dcom::Config>,
    pub wmi_method: Option<WmiMethod>,
    pub retries: Option<u8>,
    pub quircks: WmiQuircks,
}

impl Config {
    pub async fn get_session(
        &self,
        key_vault: &KeyVault,
    ) -> Result<WmiSession> {
        if let Some(cnf) = &self.powershell {
            cnf.new_session(key_vault)
                .await
                .map_err(WMIError::WinRMProtError)
                .map(WmiSession::Powershell)
        } else if let Some(cnf) = &self.dcom {
            cnf.new_session(key_vault).await.map(WmiSession::Dcom)
        } else {
            Err(WMIError::NoConnectionConfig)
        }
    }

    pub fn get_method(&self) -> WmiMethod {
        self.wmi_method.unwrap_or(WmiMethod::GetWmiObject)
    }
}

pub enum WmiSession {
    Powershell(ps::WindowsSession),
    Dcom(dcom::DcomSession),
}

impl WmiSession {
    pub async fn get_wmiobject(
        &mut self,
        class: &str,
        namespace: &str,
        attributes: &[String],
    ) -> DTResult<Vec<HashMap<String, String>>> {
        match self {
            Self::Powershell(ps) => ps
                .get_wmiobject(class, namespace, attributes)
                .await
                .map_err(WMIDTError::Powershell),
            Self::Dcom(dcom) => {
                dcom.get_wmiobject(class, namespace, attributes).await
            }
        }
    }

    pub async fn get_ciminstance(
        &mut self,
        class: &str,
        namespace: &str,
        attributes: &[String],
    ) -> DTResult<Vec<HashMap<String, String>>> {
        match self {
            Self::Powershell(ps) => ps
                .get_ciminstance(class, namespace, attributes)
                .await
                .map_err(WMIDTError::Powershell),
            Self::Dcom(dcom) => {
                dcom.get_wmiobject(class, namespace, attributes).await
            }
        }
    }

    pub async fn enumerate_ciminstance(
        &mut self,
        class: &str,
        namespace: &str,
        attributes: &[String],
    ) -> DTResult<Vec<HashMap<String, String>>> {
        match self {
            Self::Powershell(ps) => ps
                .enumerate_ciminstance(class, namespace, attributes)
                .await
                .map_err(WMIDTError::Powershell),
            Self::Dcom(dcom) => {
                dcom.get_wmiobject(class, namespace, attributes).await
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub enum WmiMethod {
    GetWmiObject,
    GetCimInstance,
    EnumerateCimInstance,
}

impl WmiMethod {
    pub fn requires_shell(&self) -> bool {
        matches!(self, WmiMethod::GetWmiObject | WmiMethod::GetCimInstance)
    }

    pub async fn exec_query(
        &self,
        session: &mut WmiSession,
        classname: &str,
        properties: &[String],
        namespace: &str,
    ) -> DTResult<Vec<HashMap<String, String>>> {
        match self {
            WmiMethod::EnumerateCimInstance => {
                session
                    .enumerate_ciminstance(classname, namespace, properties)
                    .await
            }
            WmiMethod::GetCimInstance => {
                session
                    .get_ciminstance(classname, namespace, properties)
                    .await
            }
            WmiMethod::GetWmiObject => {
                session
                    .get_wmiobject(classname, namespace, properties)
                    .await
            }
        }
        .map_err(|e| WMIDTError::Request(e.to_string()))
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct WmiQuircks {
    pub local_as_utc: Option<LocalAsUtcQuirck>,
}

impl WmiQuircks {
    pub fn get_tz(
        &self,
        class: &String,
        property: &String,
    ) -> Option<Result<chrono_tz::Tz>> {
        self.local_as_utc
            .as_ref()
            .and_then(|q| q.valid(class, property))
            .map(|q| q.timezone())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LocalAsUtcQuirck {
    fields: HashMap<String, Vec<String>>,
    timezone: String,
}

impl LocalAsUtcQuirck {
    pub fn valid(
        &self,
        class: &String,
        property: &String,
    ) -> Option<&LocalAsUtcQuirck> {
        self.fields
            .get(class)
            .map(|fs| fs.iter().any(|f| f == property))
            .unwrap_or_default()
            .then_some(self)
    }
    fn timezone(&self) -> Result<chrono_tz::Tz> {
        self.timezone
            .parse::<chrono_tz::Tz>()
            .map_err(|e| WMIError::TimeZoneParse(self.timezone.to_string(), e))
    }
}
