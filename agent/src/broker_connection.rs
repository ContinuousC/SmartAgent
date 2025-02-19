/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use broker_api::{
    AgentToBrokerMessage, AgentToBrokerMessageCompat, BrokerToAgentMessage,
    BrokerToAgentMessageCompat,
};
use rpc::{AsyncDuplex, AsyncRequest, AsyncResponse, GenericValue};
use tokio::sync::mpsc;

pub fn broker_message_handler<V>(
    agent_sender: mpsc::Sender<AsyncRequest<V>>,
    db_sender: mpsc::Sender<AsyncResponse<V>>,
) -> impl Fn(
    BrokerToAgentMessage<V>,
) -> std::result::Result<(), AgentToBrokerMessage<V>>
where
    V: GenericValue,
{
    move |msg: BrokerToAgentMessage<V>| match msg {
        BrokerToAgentMessage::Backend { message } => {
            agent_sender.try_send(message).map_err(|e| {
                log::warn!("failed to send agent request: {}", e);
                AgentToBrokerMessage::Backend {
                    message: AsyncResponse::<V>::from_send_error::<String>(e),
                }
            })
        }
        BrokerToAgentMessage::MetricsEngine { message } => {
            if let Err(e) = db_sender.try_send(message) {
                log::warn!("failed to send db response: {}", e);
            }
            Ok(())
        }
    }
}

pub fn broker_message_handler_compat<V>(
    agent_sender: mpsc::Sender<AsyncRequest<V>>,
    db_sender: mpsc::Sender<AsyncResponse<V>>,
) -> impl Fn(
    BrokerToAgentMessageCompat<V>,
) -> std::result::Result<(), AgentToBrokerMessageCompat<V>>
where
    V: GenericValue,
{
    move |msg: BrokerToAgentMessageCompat<V>| match msg {
        BrokerToAgentMessageCompat::Backend {
            message: AsyncDuplex::Request(message),
        } => agent_sender.try_send(message).map_err(|e| {
            log::warn!("failed to send agent request: {}", e);
            AgentToBrokerMessageCompat::Backend {
                message: AsyncDuplex::Response(
                    AsyncResponse::<V>::from_send_error::<String>(e),
                ),
            }
        }),
        BrokerToAgentMessageCompat::Database {
            message: AsyncDuplex::Response(message),
        } => {
            if let Err(e) = db_sender.try_send(message) {
                log::warn!("failed to send db response: {}", e);
            }
            Ok(())
        }
        _ => {
            log::warn!("dropping unexpected message from broker.");
            Ok(())
        }
    }
}

pub fn broker_unconnected_handler<V: GenericValue + Clone>(
    db_sender: mpsc::Sender<AsyncResponse<V>>,
) -> impl Fn(AgentToBrokerMessage<V>) {
    let unconnected =
        V::serialize_from(Result::<(), &str>::Err("not connected!")).unwrap();
    move |msg: AgentToBrokerMessage<V>| match msg {
        AgentToBrokerMessage::Backend { message: _ } => {
            log::warn!("failed to send response to backend: not connected!");
        }
        AgentToBrokerMessage::MetricsEngine {
            message: AsyncRequest { req_id, request: _ },
        } => {
            if let Err(e) = db_sender.try_send(AsyncResponse {
                req_id,
                response: unconnected.clone(),
            }) {
                log::warn!("failed to send db response: {}", e);
            }
        }
    }
}

pub fn broker_unconnected_handler_compat<V: GenericValue + Clone>(
    db_sender: mpsc::Sender<AsyncResponse<V>>,
) -> impl Fn(AgentToBrokerMessageCompat<V>) {
    let unconnected =
        V::serialize_from(Result::<(), &str>::Err("not connected!")).unwrap();
    move |msg: AgentToBrokerMessageCompat<V>| match msg {
        AgentToBrokerMessageCompat::Backend { message: _ } => {
            log::warn!("failed to send response to backend: not connected!");
        }
        AgentToBrokerMessageCompat::Database {
            message: AsyncDuplex::Request(AsyncRequest { req_id, request: _ }),
        } => {
            if let Err(e) = db_sender.try_send(AsyncResponse {
                req_id,
                response: unconnected.clone(),
            }) {
                log::warn!("failed to send db response: {}", e);
            }
        }
        _ => {
            log::warn!("dropping unexpected message from broker");
        }
    }
}
