/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::{hash_map::Entry, HashMap},
    marker::PhantomData,
};

use broker_api::{
    AgentConnectionStatus, AgentId, AgentToBrokerMessage, BrokerEvent,
    BrokerToAgentMessage, BrokerToBackendMessage, OrgId,
};
use chrono::Utc;
use rpc::{
    AsyncResponse, BrokerHandler, CborReadStream, CborStream, CborWriteStream,
    MsgStream, TlsStreamExt,
};
use serde_cbor::Value;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tokio_rustls::server::TlsStream;

use crate::node::Node;

pub struct AgentHandler<V>(PhantomData<V>);

impl<V> AgentHandler<V> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}
impl<S> BrokerHandler<TlsStream<S>> for AgentHandler<Value>
where
    S: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    type Key = (OrgId, AgentId);
    type Node = Node<Value>;
    type ReadStream = CborReadStream<TlsStream<S>, Self::ReadMsg>;
    type WriteStream = CborWriteStream<TlsStream<S>, Self::WriteMsg>;
    type ReadMsg = AgentToBrokerMessage<Value>;
    type WriteMsg = BrokerToAgentMessage<Value>;

    fn get_key(&self, stream: &TlsStream<S>) -> rpc::Result<Self::Key> {
        let (org, cn) =
            stream.peer_org_and_cn().ok_or(rpc::Error::Authentication)?;
        Ok((OrgId(org), AgentId(cn)))
    }

    fn add_node(
        &self,
        nodes: &mut HashMap<OrgId, Self::Node>,
        (org, agent): &Self::Key,
        sender: mpsc::Sender<Self::WriteMsg>,
    ) -> rpc::Result<()> {
        let node = nodes.entry(org.clone()).or_insert_with(Self::Node::default);
        match node.agents.entry(agent.clone()) {
            Entry::Occupied(_) => return Err(rpc::Error::Authentication),
            Entry::Vacant(ent) => {
                ent.insert(sender);
            }
        }

        node.agent_connection_info.insert(
            agent.clone(),
            AgentConnectionStatus::Connected { since: Utc::now() },
        );

        if let Some(backend) = &node.backend {
            if let Err(e) =
                backend.try_send(BrokerToBackendMessage::BrokerEvent {
                    event: BrokerEvent::AgentConnected {
                        agent_id: agent.clone(),
                    },
                })
            {
                log::debug!(
                    "failed to send agent connected event for agent {}: {}",
                    &agent.0,
                    e
                );
            } else {
                log::debug!(
                    "successfully sent agent connected event for agent {}",
                    &agent.0,
                );
            }
        }

        log::info!("Connection from agent {}/{}", &org.0, &agent.0);
        Ok(())
    }

    fn remove_node(
        &self,
        nodes: &mut HashMap<OrgId, Self::Node>,
        (org, agent): &Self::Key,
    ) {
        log::info!("Disconnect from agent {}/{}", &org.0, &agent.0);
        if let Some(node) = nodes.get_mut(org) {
            node.agents.remove(agent);
            node.agent_connection_info.insert(
                agent.clone(),
                AgentConnectionStatus::Disconnected {
                    since: Utc::now(),
                    error: None,
                    next_try: None,
                },
            );

            if let Some(backend) = &node.backend {
                if let Err(e) =
                    backend.try_send(BrokerToBackendMessage::BrokerEvent {
                        event: BrokerEvent::AgentDisconnected {
                            agent_id: agent.clone(),
                        },
                    })
                {
                    log::debug!(
                        "failed to send agent disconnected event for agent {}: {}",
                        &agent.0,
                        e
                    );
                } else {
                    log::debug!(
						"successfully sent agent disconnected event for agent {}",
                        &agent.0,
                    );
                }
            }
        }
    }

    fn get_node<'a>(
        &self,
        nodes: &'a HashMap<OrgId, Self::Node>,
        (org, _agent): &Self::Key,
    ) -> Option<&'a Self::Node> {
        nodes.get(org)
    }

    fn make_msg_stream(
        &self,
        stream: TlsStream<S>,
    ) -> (Self::ReadStream, Self::WriteStream) {
        CborStream::new(stream).split()
    }

    fn handle_message(
        &self,
        node: &Self::Node,
        (_, agent_id): &Self::Key,
        msg: Self::ReadMsg,
    ) -> std::result::Result<(), Self::WriteMsg> {
        match msg {
            AgentToBrokerMessage::Backend { message } => match &node.backend {
                Some(backend) => {
                    match backend.try_send(BrokerToBackendMessage::Agent {
                        agent_id: agent_id.clone(),
                        message,
                    }) {
                        Ok(()) => Ok(()),
                        Err(_) => Ok(()), // ignore msg when queue is full
                    }
                }
                None => Ok(()), // ignore msg when backend is not connected
            },
            AgentToBrokerMessage::MetricsEngine { message } => {
                let req_id = message.req_id;
                match &node.database {
                    Some(db) => {
                        match db.try_send(
                            broker_api::BrokerToMetricsEngineMessage::Agent {
                                agent_id: agent_id.clone(),
                                message,
                            },
                        ) {
                            Ok(()) => Ok(()),
                            Err(_) => {
                                Err(BrokerToAgentMessage::MetricsEngine {
                                    message: AsyncResponse {
                                        req_id,
                                        response:
                                            serde_cbor::value::to_value::<
                                                std::result::Result<(), &str>,
                                            >(
                                                Err("dbdaemon queue full")
                                            )
                                            .unwrap(),
                                    },
                                })
                            }
                        }
                    }
                    None => Err(BrokerToAgentMessage::MetricsEngine {
                        message: AsyncResponse {
                            req_id,
                            response: serde_cbor::value::to_value::<
                                std::result::Result<(), &str>,
                            >(Err(
                                "metrics-engine not connected",
                            ))
                            .unwrap(),
                        },
                    }),
                }
            }
        }
    }
}
