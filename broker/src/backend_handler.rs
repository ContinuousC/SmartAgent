/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, marker::PhantomData, sync::Arc};

use serde_cbor::Value;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tokio_rustls::server::TlsStream;

use broker_api::{
    BackendToBrokerMessage, BrokerEvent, BrokerProto, BrokerToAgentMessage,
    BrokerToBackendMessage, OrgId,
};
use rpc::{
    AsyncRequest, AsyncResponse, BrokerHandler, CborReadStream, CborStream,
    CborWriteStream, GenericValue, MsgStream, RequestHandler, TlsStreamExt,
};

use crate::node::Node;

pub struct BackendHandler<H, V> {
    broker_handler: Arc<H>,
    _marker: PhantomData<V>,
}

impl<H, V> BackendHandler<H, V>
where
    H: RequestHandler<BrokerProto, V> + Send + Sync + 'static,
    V: GenericValue + Send,
    V::Error: Send,
    H::Error: Send,
{
    pub fn new(broker_handler: Arc<H>) -> Self {
        Self {
            broker_handler,
            _marker: PhantomData,
        }
    }
}

impl<H, S> BrokerHandler<TlsStream<S>> for BackendHandler<H, Value>
where
    H: RequestHandler<BrokerProto, Value, ExtraArgs = (OrgId,)>
        + Send
        + Sync
        + 'static,
    S: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
    H::Error: Send,
{
    type Key = OrgId;
    type Node = Node<Value>;
    type ReadStream = CborReadStream<TlsStream<S>, Self::ReadMsg>;
    type WriteStream = CborWriteStream<TlsStream<S>, Self::WriteMsg>;
    type ReadMsg = BackendToBrokerMessage<Value>;
    type WriteMsg = BrokerToBackendMessage<Value>;

    fn get_key(&self, stream: &TlsStream<S>) -> rpc::Result<Self::Key> {
        let org = stream
            .peer_organization()
            .ok_or(rpc::Error::Authentication)?;
        Ok(OrgId(org))
    }

    fn add_node(
        &self,
        nodes: &mut HashMap<OrgId, Self::Node>,
        org: &Self::Key,
        sender: mpsc::Sender<Self::WriteMsg>,
    ) -> rpc::Result<()> {
        let node = nodes.entry(org.clone()).or_insert_with(Self::Node::default);
        match node.backend.is_some() {
            true => {
                log::warn!(
                    "Refusing duplicate backend connection for {}",
                    &org.0
                );
                Err(rpc::Error::AuthenticationFailed)
            }
            false => {
                for agent_id in node.agents.keys() {
                    if let Err(e) =
                        sender.try_send(BrokerToBackendMessage::BrokerEvent {
                            event: BrokerEvent::AgentConnected {
                                agent_id: agent_id.clone(),
                            },
                        })
                    {
                        log::warn!(
                            "failed to send agent connected event to backend: {}",
                            e
                        );
                    }
                }
                let _ = node.backend.insert(sender);
                log::info!("Connection from backend {}", &org.0);
                Ok(())
            }
        }
    }

    fn remove_node(
        &self,
        nodes: &mut HashMap<OrgId, Self::Node>,
        org: &Self::Key,
    ) {
        if let Some(node) = nodes.get_mut(org) {
            node.backend.take();
            log::info!("Disconnect from backend {}", &org.0);
        }
    }

    fn get_node<'a>(
        &self,
        nodes: &'a HashMap<OrgId, Self::Node>,
        org: &Self::Key,
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
        org: &Self::Key,
        msg: Self::ReadMsg,
    ) -> std::result::Result<(), Self::WriteMsg> {
        match msg {
            BackendToBrokerMessage::Agent { agent_id, message } => {
                let req_id = message.req_id;
                match node.agents.get(&agent_id) {
                    Some(agent) => {
                        match agent
                            .try_send(BrokerToAgentMessage::Backend { message })
                        {
                            Ok(()) => Ok(()),
                            Err(_) => Err(BrokerToBackendMessage::Agent {
                                agent_id,
                                message: AsyncResponse {
                                    req_id,
                                    response: serde_cbor::value::to_value::<
                                        std::result::Result<(), &str>,
                                    >(
                                        Err("agent queue full")
                                    )
                                    .unwrap(),
                                },
                            }),
                        }
                    }
                    None => Err(BrokerToBackendMessage::Agent {
                        agent_id,
                        message: AsyncResponse {
                            req_id,
                            response: serde_cbor::value::to_value::<
                                std::result::Result<(), &str>,
                            >(Err(
                                "agent not connected",
                            ))
                            .unwrap(),
                        },
                    }),
                }
            }
            BackendToBrokerMessage::Broker {
                message: AsyncRequest { req_id, request },
            } => {
                let res_sender = match node.backend.as_ref() {
                    Some(s) => s.clone(),
                    None => return Ok(()),
                };
                let handler = self.broker_handler.clone();
                let org_id = org.clone();
                tokio::spawn(async move {
                    match handler.handle(request, (org_id,)).await {
                        Ok(response) => {
                            if let Err(e) = res_sender
                                .send(BrokerToBackendMessage::Broker {
                                    message: AsyncResponse { req_id, response },
                                })
                                .await
                            {
                                log::debug!("failed to send broker response to backend: {}", e);
                            }
                        }
                        Err(e) => {
                            log::debug!("broker handler failed: {}", e);
                        }
                    }
                });
                Ok(())
            }
        }
    }
}
