/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::sync::Arc;

use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, watch};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::server::TlsStream;

use api::{AgentId, OrgId};
use rpc::{handshake_server, HandshakeClient, HandshakeServer};
use rpc::{CborStream, MsgStream};

use super::agent_handler::{accept_agent, AgentCmd};
use super::backend_handler::{accept_backend, BackendCmd};
use super::broker_service::BrokerService;
use super::database_handler::{accept_database, DatabaseCmd};
use super::error::{Error, Result};
use super::tls::get_certificate_ids;

/// Messages sent from the agent, backend or database connection handler
/// to the broker.
#[derive(Debug)]
pub enum BrokerCmd {
    RegisterAgent(
        OrgId,
        AgentId,
        mpsc::Sender<AgentCmd>,
        oneshot::Sender<watch::Receiver<Node>>,
    ),
    RegisterBackend(
        OrgId,
        mpsc::Sender<BackendCmd>,
        oneshot::Sender<watch::Receiver<Node>>,
    ),
    RegisterDatabase(
        OrgId,
        mpsc::Sender<DatabaseCmd>,
        oneshot::Sender<watch::Receiver<Node>>,
    ),

    UnregisterAgent(OrgId, AgentId),
    UnregisterBackend(OrgId),
    UnregisterDatabase(OrgId),

    /* TODO: handle BrokerService in backend_handler? */
    AgentConnected(OrgId, AgentId, oneshot::Sender<bool>),
    AgentConnection(OrgId, AgentId, ssh::Forward),
    DisconnectAgent(OrgId, AgentId),
}

pub async fn accept_connections(
    tls_config: Arc<ServerConfig>,
    agent_addr: String,
    backend_addr: String,
    database_addr: String,
    server_name: String,
    server_port: u32,
) -> Result<()> {
    let agent_listener = TcpListener::bind(agent_addr)
        .await
        .map_err(Error::AgentListener)?;
    let backend_listener = TcpListener::bind(backend_addr)
        .await
        .map_err(Error::BackendListener)?;
    let database_listener = TcpListener::bind(database_addr)
        .await
        .map_err(Error::DatabaseListener)?;

    let (sender, receiver) = mpsc::channel(1000);

    let broker_service =
        Arc::new(BrokerService::new(sender.clone(), server_name, server_port));

    // Unless an irrecoverable error occurs, these futures should not return.
    tokio::select! {
        r = accept_agents(agent_listener, tls_config.clone(), sender.clone()) => r,
        r = accept_backends(backend_listener, tls_config.clone(), sender.clone(),
                broker_service) => r,
        r = accept_databases(database_listener, tls_config.clone(), sender.clone()) => r,
        r = handle_commands(receiver, tls_config, sender) => r,
    }
}

async fn accept_agents(
    agent_listener: TcpListener,
    tls_config: Arc<ServerConfig>,
    sender: mpsc::Sender<BrokerCmd>,
) -> Result<()> {
    loop {
        let (stream, _addr) = agent_listener
            .accept()
            .await
            .map_err(Error::AgentListener)?;
        let sender = sender.clone();
        accept_connection(
            stream,
            tls_config.clone(),
            |stream| CborStream::new(BufReader::new(stream)),
            move |stream, org, cn, _version| {
                accept_agent(stream, sender, OrgId(org), AgentId(cn))
            },
        );
    }
}

async fn accept_backends(
    backend_listener: TcpListener,
    tls_config: Arc<ServerConfig>,
    sender: mpsc::Sender<BrokerCmd>,
    broker_service: Arc<BrokerService>,
) -> Result<()> {
    loop {
        let (stream, _addr) = backend_listener
            .accept()
            .await
            .map_err(Error::BackendListener)?;
        let sender = sender.clone();
        let broker_service = broker_service.clone();
        accept_connection(
            stream,
            tls_config.clone(),
            |stream| CborStream::new(BufReader::new(stream)),
            move |stream, org, _cn, _version| {
                accept_backend(stream, sender, OrgId(org), broker_service)
            },
        );
    }
}

async fn accept_databases(
    database_listener: TcpListener,
    tls_config: Arc<ServerConfig>,
    sender: mpsc::Sender<BrokerCmd>,
) -> Result<()> {
    loop {
        let (stream, _addr) = database_listener
            .accept()
            .await
            .map_err(Error::DatabaseListener)?;
        let sender = sender.clone();
        accept_connection(
            stream,
            tls_config.clone(),
            |stream| CborStream::new(BufReader::new(stream)),
            move |stream, org, _cn, _version| {
                accept_database(stream, sender, OrgId(org))
            },
        );
    }
}

fn accept_connection<S, T, F, G, R>(
    stream: S,
    tls: Arc<ServerConfig>,
    wrap: F,
    accept: G,
) where
    S: AsyncRead + AsyncWrite + Send + Unpin + 'static,
    T: MsgStream<HandshakeServer, HandshakeClient> + Send,
    F: Fn(TlsStream<S>) -> T + Send + 'static,
    G: FnOnce(T, String, String, u64) -> R + Send + 'static,
    R: Future<Output = Result<()>> + Send,
{
    tokio::spawn(async move {
        //eprintln!("Incoming connection from {}...", addr);
        match tokio_rustls::TlsAcceptor::from(tls).accept(stream).await {
            Ok(stream) => {
                let (org, cn) = match get_certificate_ids(&stream) {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("Invalid certificate: {}", e);
                        return;
                    }
                };
                let mut stream = (wrap)(stream);
                let version = match handshake_server(&mut stream, 0, 0).await {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Handshake failed: {}", e);
                        return;
                    }
                };
                //eprintln!("Handshake succeeded (version = {}", version);
                if let Err(e) = (accept)(stream, org, cn, version).await {
                    eprintln!("Connection handler failed: {}", e);
                }
            }
            Err(e) => {
                eprintln!("TLS negotiation failed: {}", e);
            }
        }
    });
}

#[derive(Clone, Default, Debug)]
pub struct Node {
    pub backend: Option<Arc<mpsc::Sender<BackendCmd>>>,
    pub database: Option<Arc<mpsc::Sender<DatabaseCmd>>>,
    pub agents: HashMap<AgentId, Arc<mpsc::Sender<AgentCmd>>>,
}

impl Node {
    fn is_empty(&self) -> bool {
        self.backend.is_none()
            && self.database.is_none()
            && self.agents.is_empty()
    }
}

async fn handle_commands(
    mut receiver: mpsc::Receiver<BrokerCmd>,
    tls_config: Arc<ServerConfig>,
    sender: mpsc::Sender<BrokerCmd>,
) -> Result<()> {
    let mut nodes: HashMap<
        OrgId,
        (watch::Sender<Node>, watch::Receiver<Node>),
    > = HashMap::new();

    loop {
        match receiver.recv().await.ok_or(Error::BrokerChannelClosed)? {
            BrokerCmd::AgentConnected(org_id, agent_id, response) => {
                let connected =
                    nodes.get(&org_id).map_or(false, |(node_sender, _)| {
                        node_sender.borrow().agents.contains_key(&agent_id)
                    });
                let _ = response.send(connected);
            }
            BrokerCmd::RegisterAgent(
                org_id,
                agent_id,
                sender,
                node_receiver_response,
            ) => {
                let (node_sender, node_receiver) = nodes
                    .entry(org_id.clone())
                    .or_insert_with(|| watch::channel(Node::default()));
                let mut node = node_sender.borrow().clone();
                if let Some(_old) =
                    node.agents.insert(agent_id.clone(), Arc::new(sender))
                {
                    eprintln!(
                        "Warning: new agent connection handler \
						 registered for {} / {} without unregister \
						 from the old one!",
                        org_id.0, agent_id.0
                    );
                }
                let _ = node_sender.send(node);
                let _ = node_receiver_response.send(node_receiver.clone());
            }
            BrokerCmd::UnregisterAgent(org_id, agent_id) => {
                match nodes.get_mut(&org_id) {
					Some((node_sender, _)) => {
						let mut node = node_sender.borrow().clone();
						match node.agents.remove(&agent_id) {
							Some(_sender) => match node.is_empty() {
								true => { nodes.remove(&org_id); },
								false => { let _ = node_sender.send(node); }
							}
							None => eprintln!("Warning got unregister command for \
											   unregistered agent {} / {}",
											  org_id.0, agent_id.0)
						}
					},
					None => eprintln!("Warning got unregister command for unregistered agent \
									   {} / {}", org_id.0, agent_id.0)
				}
            }

            BrokerCmd::RegisterBackend(
                org_id,
                sender,
                node_receiver_response,
            ) => {
                let (node_sender, node_receiver) = nodes
                    .entry(org_id.clone())
                    .or_insert_with(|| watch::channel(Node::default()));
                let mut node = node_sender.borrow().clone();
                if let Some(_old) = node.backend.replace(Arc::new(sender)) {
                    eprintln!(
                        "Warning: new backend connection handler \
						 registered for {} without unregister from \
						 the old one!",
                        org_id.0
                    );
                }
                let _ = node_sender.send(node);
                let _ = node_receiver_response.send(node_receiver.clone());
            }
            BrokerCmd::UnregisterBackend(org_id) => {
                match nodes.get_mut(&org_id) {
                    Some((node_sender, _)) => {
                        let mut node = node_sender.borrow().clone();
                        if node.backend.take().is_none() {
                            eprintln!("Warning got unregister command for unregistered backend \
									   {}", org_id.0);
                        }
                        match node.is_empty() {
                            true => {
                                nodes.remove(&org_id);
                            }
                            false => {
                                let _ = node_sender.send(node);
                            }
                        }
                    }
                    None => {
                        eprintln!("Warning got unregister backend command for unregistered node \
			       {}", org_id.0);
                    }
                }
            }

            BrokerCmd::RegisterDatabase(
                org_id,
                sender,
                node_receiver_response,
            ) => {
                let (node_sender, node_receiver) = nodes
                    .entry(org_id.clone())
                    .or_insert_with(|| watch::channel(Node::default()));
                let mut node = node_sender.borrow().clone();
                if let Some(_old) = node.database.replace(Arc::new(sender)) {
                    eprintln!("Warning: new database connection handler registered for {} \
			       without unregister from the old one!", org_id.0);
                }
                let _ = node_sender.send(node);
                let _ = node_receiver_response.send(node_receiver.clone());
            }
            BrokerCmd::UnregisterDatabase(org_id) => {
                match nodes.get_mut(&org_id) {
                    Some((node_sender, _)) => {
                        let mut node = node_sender.borrow().clone();
                        if node.database.take().is_none() {
                            eprintln!("Warning got unregister command for unregistered db daemon \
				   {}", org_id.0);
                        }
                        match node.is_empty() {
                            true => {
                                nodes.remove(&org_id);
                            }
                            false => {
                                let _ = node_sender.send(node);
                            }
                        }
                    }
                    None => {
                        eprintln!("Warning got unregister db daemon command for unregistered node \
			       {}", org_id.0);
                    }
                }
            }

            BrokerCmd::AgentConnection(org_id, agent_id, stream) => {
                let sender = sender.clone();
                accept_connection(
                    stream,
                    tls_config.clone(),
                    |stream| CborStream::new(BufReader::new(stream)),
                    move |stream, org, cn, _version| async move {
                        match org_id.0 == org && agent_id.0 == cn {
                            true => {
                                accept_agent(
                                    stream,
                                    sender,
                                    OrgId(org),
                                    AgentId(cn),
                                )
                                .await
                            }
                            false => {
                                eprintln!(
                                    "Found the wrong agent: {} =?= {} / {} =?= {}",
                                    org_id.0, org, agent_id.0, cn
                                );
                                Err(Error::Authentication)
                            }
                        }
                    },
                );
            }
            BrokerCmd::DisconnectAgent(org_id, agent_id) => {
                match nodes.get_mut(&org_id) {
                    Some((node_sender, _)) => {
                        let mut node = node_sender.borrow().clone();
                        match node.agents.remove(&agent_id) {
                            Some(agent) => {
                                agent.send(AgentCmd::Disconnect).await?
                            }
                            None => eprintln!(
                                "Warning: got disconnect command for \
								 unconnected agent {} / {}",
                                org_id.0, agent_id.0
                            ),
                        }
                        let _ = node_sender.send(node);
                    }
                    None => eprintln!(
                        "Warning: got agent disconnect command from \
						 unconnected backend {}",
                        org_id.0
                    ),
                }
            }
        }
    }
}
