/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use broker_api::{
    AgentConnectionStatus, AgentId, BrokerToAgentMessage,
    BrokerToBackendMessage, BrokerToMetricsEngineMessage, OrgId,
};
use tokio::sync::mpsc;

#[derive(Clone, Debug)]
pub struct Node<V> {
    pub backend: Option<mpsc::Sender<BrokerToBackendMessage<V>>>,
    pub database: Option<mpsc::Sender<BrokerToMetricsEngineMessage<V>>>,
    pub agents: HashMap<AgentId, mpsc::Sender<BrokerToAgentMessage<V>>>,
    pub agent_connection_info: HashMap<AgentId, AgentConnectionStatus>,
}

impl<V> Default for Node<V> {
    fn default() -> Self {
        Self {
            backend: None,
            database: None,
            agents: HashMap::new(),
            agent_connection_info: HashMap::new(),
        }
    }
}

impl<V> rpc::BrokerNode for Node<V> {
    type Key = OrgId;
}
