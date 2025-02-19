/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, marker::PhantomData};

use serde_cbor::Value;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
};
use tokio_rustls::server::TlsStream;

use broker_api::{
    BrokerToAgentMessage, BrokerToMetricsEngineMessage,
    MetricsEngineToBrokerMessage, OrgId,
};
use rpc::{
    BrokerHandler, CborReadStream, CborStream, CborWriteStream, MsgStream,
    TlsStreamExt,
};

use crate::node::Node;

pub struct DatabaseHandler<V>(PhantomData<V>);

impl<V> DatabaseHandler<V> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<S> BrokerHandler<TlsStream<S>> for DatabaseHandler<Value>
where
    S: AsyncRead + AsyncWrite + Send + Sync + Unpin + 'static,
{
    type Key = OrgId;
    type Node = Node<Value>;
    type ReadStream = CborReadStream<TlsStream<S>, Self::ReadMsg>;
    type WriteStream = CborWriteStream<TlsStream<S>, Self::WriteMsg>;
    type ReadMsg = MetricsEngineToBrokerMessage<Value>;
    type WriteMsg = BrokerToMetricsEngineMessage<Value>;

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
        let ent = &mut nodes
            .entry(org.clone())
            .or_insert_with(Self::Node::default)
            .database;
        match ent.is_some() {
            true => {
                log::warn!(
                    "Refusing duplicate database connection for {}",
                    &org.0
                );
                Err(rpc::Error::AuthenticationFailed)
            }
            false => {
                let _ = ent.insert(sender);
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
            node.database.take();
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
        _org: &Self::Key,
        msg: Self::ReadMsg,
    ) -> std::result::Result<(), Self::WriteMsg> {
        match msg {
            MetricsEngineToBrokerMessage::Agent { agent_id, message } => {
                match node.agents.get(&agent_id) {
                    Some(agent) => {
                        match agent.try_send(
                            BrokerToAgentMessage::MetricsEngine { message },
                        ) {
                            Ok(()) => Ok(()),
                            Err(_) => Ok(()),
                        }
                    }
                    None => Ok(()),
                }
            }
        }
    }
}
