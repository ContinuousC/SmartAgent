/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::fmt::{self, Display};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use rpc::rpc;

use crate::{AgentId, OrgId};

#[rpc(
    service(extra_args = "backend_id: OrgId"),
    stub(javascript(req_method = "broker_request"))
)]
pub trait BrokerService {
    // SSH connection management.
    async fn ssh_connections(&self) -> HashMap<AgentId, SshConfig>;
    async fn connect_agent(&self, agent_id: AgentId, ssh_config: SshConfig);
    async fn disconnect_agent(&self, agent_id: AgentId);

    // Connection status.
    async fn get_connected_agents(
        &self,
    ) -> HashMap<AgentId, AgentConnectionInfo>;
    async fn get_agent_conn_status(
        &self,
        agent_id: AgentId,
    ) -> Option<AgentConnectionInfo>;
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
pub struct SshConfig {
    pub host: String,
    pub jump_hosts: Vec<String>,
    pub known_hosts: HashMap<String, String>,
    pub private_key: String,
    pub agent_port: u32,
    pub retry_interval: Option<f64>,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
pub struct AgentConnectionInfo {
    pub conn_type: AgentConnectionType,
    pub status: AgentConnectionStatus,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AgentConnectionStatus {
    Connected {
        since: DateTime<Utc>,
    },
    Disconnected {
        since: DateTime<Utc>,
        error: Option<String>,
        next_try: Option<DateTime<Utc>>,
    },
    Retrying,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Clone, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AgentConnectionType {
    Direct,
    Ssh,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum BrokerEvent {
    AgentConnected { agent_id: AgentId },
    AgentDisconnected { agent_id: AgentId },
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BrokerError {
    /// Whether a retry might succeed without user intervention.
    pub retry: bool,
    /// The error message
    pub message: String,
}

impl Display for BrokerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<String> for BrokerError {
    fn from(err: String) -> Self {
        Self {
            retry: false,
            message: err,
        }
    }
}
