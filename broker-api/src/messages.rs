/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use serde::{Deserialize, Serialize};

use crate::{service::BrokerEvent, AgentId};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BackendToBrokerMessage<Value> {
    Agent {
        agent_id: AgentId,
        message: rpc::AsyncRequest<Value>,
    },
    Broker {
        message: rpc::AsyncRequest<Value>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum BrokerToBackendMessage<Value> {
    Agent {
        agent_id: AgentId,
        message: rpc::AsyncResponse<Value>,
    },
    Broker {
        message: rpc::AsyncResponse<Value>,
    },
    BrokerEvent {
        event: BrokerEvent,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AgentToBrokerMessage<Value> {
    Backend { message: rpc::AsyncResponse<Value> },
    MetricsEngine { message: rpc::AsyncRequest<Value> },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BrokerToAgentMessage<Value> {
    Backend { message: rpc::AsyncRequest<Value> },
    MetricsEngine { message: rpc::AsyncResponse<Value> },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BrokerToAgentMessageCompat<Value> {
    Backend {
        message: rpc::AsyncDuplex<Value, Value, Value>,
    },
    Database {
        message: rpc::AsyncDuplex<Value, Value, Value>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum AgentToBrokerMessageCompat<Value> {
    Backend {
        message: rpc::AsyncDuplex<Value, Value, Value>,
    },
    Database {
        message: rpc::AsyncDuplex<Value, Value, Value>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MetricsEngineToBrokerMessage<Value> {
    Agent {
        agent_id: AgentId,
        message: rpc::AsyncResponse<Value>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum BrokerToMetricsEngineMessage<Value> {
    Agent {
        agent_id: AgentId,
        message: rpc::AsyncRequest<Value>,
    },
}
