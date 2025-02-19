/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::{collections::HashMap, sync::RwLock};

use async_trait::async_trait;

use broker_api::{
    AgentConnectionInfo, AgentConnectionStatus, AgentConnectionType, AgentId,
    OrgId, SshConfig,
};
use rpc::NodeMap;
use serde_cbor::Value;
use tokio_rustls::rustls::ServerConfig;

use crate::error::{Error, Result};
use crate::node::Node;
use crate::ssh_connector::SshConnector;

pub struct BrokerService {
    ssh_config: RwLock<HashMap<OrgId, HashMap<AgentId, SshConfig>>>,
    ssh_connectors: RwLock<HashMap<OrgId, HashMap<AgentId, SshConnector>>>,
    nodes: Arc<RwLock<NodeMap<Node<Value>>>>,
    tls_config: Arc<ServerConfig>,
    server_name: String,
    server_port: u32,
}

impl BrokerService {
    pub fn new(
        ssh_config: HashMap<OrgId, HashMap<AgentId, SshConfig>>,
        nodes: Arc<RwLock<NodeMap<Node<Value>>>>,
        tls_config: Arc<ServerConfig>,
        server_name: String,
        server_port: u32,
    ) -> Self {
        Self {
            nodes,
            tls_config,
            server_name,
            server_port,
            ssh_config: RwLock::new(ssh_config),
            ssh_connectors: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl broker_api::BrokerService for BrokerService {
    type Error = Error;

    async fn ssh_connections(
        &self,
        backend_id: OrgId,
    ) -> Result<HashMap<AgentId, SshConfig>> {
        Ok(self
            .ssh_config
            .read()
            .unwrap()
            .get(&backend_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn connect_agent(
        &self,
        org_id: OrgId,
        agent_id: AgentId,
        ssh_config: SshConfig,
    ) -> Result<()> {
        log::info!(
            "Adding/updating ssh connection to agent on host {}...",
            &ssh_config.host
        );

        let old_config = self
            .ssh_config
            .write()
            .unwrap()
            .entry(org_id.clone())
            .or_insert_with(HashMap::new)
            .insert(agent_id.clone(), ssh_config.clone());

        if old_config.map_or(false, |c| c == ssh_config) {
            log::debug!("SSH config unchanged --> success!");
            return Ok(());
        }

        let old_connector = self
            .ssh_connectors
            .write()
            .unwrap()
            .entry(org_id.clone())
            .or_insert_with(HashMap::new)
            .remove(&agent_id);

        if let Some(connector) = old_connector {
            if let Err(e) = connector.shutdown().await {
                log::debug!("SSH connector failed: {}", e);
            }
        }

        self.ssh_connectors
            .write()
            .unwrap()
            .entry(org_id.clone())
            .or_insert_with(HashMap::new)
            .insert(
                agent_id.clone(),
                SshConnector::new(
                    org_id,
                    agent_id,
                    ssh_config,
                    self.tls_config.clone(),
                    self.server_name.to_string(),
                    self.server_port,
                    self.nodes.clone(),
                ),
            );

        Ok(())
    }

    async fn disconnect_agent(
        &self,
        org_id: OrgId,     // From backend certificate
        agent_id: AgentId, // Specified by backend
    ) -> Result<()> {
        log::info!("Disconnecting {} from {}", agent_id.0, org_id.0);
        if let Entry::Occupied(mut ent) =
            self.ssh_config.write().unwrap().entry(org_id.clone())
        {
            ent.get_mut().remove(&agent_id);
            if ent.get().is_empty() {
                ent.remove();
            }
        }

        let connector = match self.ssh_connectors.write().unwrap().entry(org_id)
        {
            Entry::Occupied(mut ent) => {
                let connector = ent.get_mut().remove(&agent_id);
                if ent.get().is_empty() {
                    ent.remove();
                }
                connector
            }
            Entry::Vacant(_) => None,
        };

        if let Some(connector) = connector {
            connector.shutdown().await?;
        }

        Ok(())
    }

    async fn get_connected_agents(
        &self,
        org_id: OrgId, // From backend certificate
    ) -> Result<HashMap<AgentId, AgentConnectionInfo>> {
        let nodes_read = self.nodes.read().unwrap();
        let ssh_config_read = self.ssh_config.read().unwrap();
        let node = nodes_read.get(&org_id).ok_or(Error::BackendNotConnected)?;
        let ssh_config = ssh_config_read.get(&org_id);
        Ok(node
            .agent_connection_info
            .iter()
            .map(|(agent_id, status)| {
                (
                    agent_id.clone(),
                    agent_connection_info(agent_id, status, &ssh_config),
                )
            })
            .collect())
    }

    async fn get_agent_conn_status(
        &self,
        org_id: OrgId, // From backend certificate
        agent_id: AgentId,
    ) -> Result<Option<AgentConnectionInfo>> {
        let nodes_read = self.nodes.read().unwrap();
        let ssh_config_read = self.ssh_config.read().unwrap();
        let node = nodes_read.get(&org_id).ok_or(Error::BackendNotConnected)?;
        let ssh_config = ssh_config_read.get(&org_id);
        Ok(node.agent_connection_info.get(&agent_id).map(|status| {
            agent_connection_info(&agent_id, status, &ssh_config)
        }))
    }
}

fn agent_connection_info(
    agent_id: &AgentId,
    status: &AgentConnectionStatus,
    ssh_config: &Option<&HashMap<AgentId, SshConfig>>,
) -> AgentConnectionInfo {
    AgentConnectionInfo {
        conn_type: match ssh_config
            .map_or(false, |conf| conf.contains_key(agent_id))
        {
            true => AgentConnectionType::Ssh,
            false => AgentConnectionType::Direct,
        },
        status: status.clone(),
    }
}
