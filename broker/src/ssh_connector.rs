/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::sync::{Arc, RwLock};

use broker_api::{AgentConnectionStatus, AgentId, OrgId, SshConfig};
use chrono::Utc;
use rpc::NodeMap;
use serde_cbor::Value;
use tokio::{sync::watch, task::JoinHandle};
use tokio_rustls::{rustls::ServerConfig, TlsAcceptor};

use rpc::TlsStreamExt;

use crate::{
    agent_handler::AgentHandler,
    error::{Error, Result},
    node::Node,
};

pub struct SshConnector {
    connector: JoinHandle<Result<()>>,
    term_sender: watch::Sender<bool>,
}

impl SshConnector {
    pub fn new(
        org_id: OrgId,
        agent_id: AgentId,
        ssh_config: SshConfig,
        tls_config: Arc<ServerConfig>,
        server_name: String,
        server_port: u32,
        nodes: Arc<RwLock<NodeMap<Node<Value>>>>,
    ) -> Self {
        let (term_sender, term_receiver) = watch::channel(false);
        Self {
            connector: tokio::spawn(ssh_connector(
                org_id,
                agent_id,
                ssh_config,
                tls_config,
                server_name,
                server_port,
                nodes,
                term_receiver,
            )),
            term_sender,
        }
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.term_sender.send(true).map_err(|_| Error::SendTerm)?;
        (&mut self.connector)
            .await
            .unwrap_or_else(|e| Err(Error::SshConnector(e)))
    }
}

impl Drop for SshConnector {
    fn drop(&mut self) {
        let _ = self.term_sender.send(true);
    }
}

async fn ssh_connector(
    org_id: OrgId,
    agent_id: AgentId,
    ssh_config: SshConfig,
    tls_config: Arc<ServerConfig>,
    server_name: String,
    server_port: u32,
    nodes: Arc<RwLock<NodeMap<Node<Value>>>>,
    mut term_receiver: watch::Receiver<bool>,
) -> Result<()> {
    let agent_handler = Arc::new(AgentHandler::<Value>::new());
    let retry_interval = ssh_config
        .retry_interval
        .map(|n| std::time::Duration::from_micros((n * 1000000.0) as u64))
        .unwrap_or_else(|| std::time::Duration::from_secs(10));

    while !*term_receiver.borrow() {
        nodes
            .write()
            .unwrap()
            .entry(org_id.clone())
            .or_insert_with(Node::default)
            .agent_connection_info
            .insert(agent_id.clone(), AgentConnectionStatus::Retrying);

        if let Err(e) = ssh_connect(
            &org_id,
            &agent_id,
            &ssh_config,
            tls_config.clone(),
            &server_name,
            server_port,
            agent_handler.clone(),
            nodes.clone(),
            term_receiver.clone(),
        )
        .await
        {
            log::error!("SSH connection failed: {}", e);
            // TODO: check if failure is recoverable

            if !*term_receiver.borrow() {
                nodes
                    .write()
                    .unwrap()
                    .entry(org_id.clone())
                    .or_insert_with(Node::default)
                    .agent_connection_info
                    .insert(
                        agent_id.clone(),
                        AgentConnectionStatus::Disconnected {
                            since: Utc::now(),
                            error: Some(e.to_string()),
                            next_try: chrono::Duration::from_std(
                                retry_interval,
                            )
                            .ok()
                            .map(|d| Utc::now() + d),
                        },
                    );
            }
        }

        rpc::abortable_sleep!(term_receiver, retry_interval);
    }

    nodes
        .write()
        .unwrap()
        .entry(org_id.clone())
        .or_insert_with(Node::default)
        .agent_connection_info
        .insert(
            agent_id.clone(),
            AgentConnectionStatus::Disconnected {
                since: Utc::now(),
                error: Some("shutting down".to_string()),
                next_try: None,
            },
        );

    Ok(())
}

async fn ssh_connect(
    org_id: &OrgId,
    agent_id: &AgentId,
    ssh_config: &SshConfig,
    tls_config: Arc<ServerConfig>,
    server_name: &str,
    server_port: u32,
    agent_handler: Arc<AgentHandler<Value>>,
    nodes: Arc<RwLock<NodeMap<Node<Value>>>>,
    term_receiver: watch::Receiver<bool>,
) -> Result<()> {
    let log_prefix = format!("{}/{}", &org_id.0, &agent_id.0);

    log::debug!("{}: establishing SSH connection", &log_prefix);

    let config = Arc::new(thrussh::client::Config::default());
    let key = Arc::new(
        thrussh_keys::decode_secret_key(ssh_config.private_key.as_str(), None)
            .map_err(Error::KeyDecode)?,
    );
    let mut session: Option<thrussh::client::Handle<ssh::Client>> = None;

    for host_arg in ssh_config
        .jump_hosts
        .iter()
        .chain(std::iter::once(&ssh_config.host))
    {
        let host = ssh::Host::parse(host_arg)
            .map_err(|e| Error::SshHostArg(host_arg.to_string(), e))?;
        let conn_string = host.conn_string();
        log::debug!("{}: Connecting to {}", &log_prefix, &conn_string);

        let mut sess = match session {
            None => thrussh::client::connect(
                config.clone(),
                &conn_string,
                ssh::Client::new(),
            )
            .await
            .map_err(|e| Error::SshConnect(host_arg.to_string(), e))?,
            Some(mut sess) => {
                let chan = sess
                    .channel_open_direct_tcpip(
                        host.host_name(),
                        host.port(),
                        server_name,
                        server_port,
                    )
                    .await
                    .map_err(|e| Error::SshChannel(host_arg.to_string(), e))?;

                thrussh::client::connect_stream(
                    config.clone(),
                    ssh::Forward::new(chan),
                    ssh::Client::new(),
                )
                .await
                .map_err(|e| Error::SshConnect(host_arg.to_string(), e))?
            }
        };

        if !sess
            .authenticate_publickey(host.user(), key.clone())
            .await
            .map_err(|e| Error::SshAuthenticate(host_arg.to_string(), e))?
        {
            return Err(Error::SshAuthentication(host_arg.to_string()));
        }

        session = Some(sess);
    }

    log::debug!("{}: forwarding agent socket", &log_prefix);

    let mut session = session.unwrap();
    let channel = session
        .channel_open_direct_tcpip(
            "localhost",
            ssh_config.agent_port,
            server_name,
            server_port,
        )
        .await
        .map_err(|e| Error::SshChannel(ssh_config.host.to_string(), e))?;

    log::debug!("{}: connecting to agent socket", &log_prefix);

    let stream = TlsAcceptor::from(tls_config)
        .accept(ssh::Forward::new(channel))
        .await
        .map_err(Error::AgentConnect)?;

    let (peer_org, peer_cn) =
        stream.peer_org_and_cn().ok_or(Error::Authentication)?;

    if peer_org != org_id.0 || peer_cn != agent_id.0 {
        return Err(Error::Authentication);
    }

    log::debug!("{}: connected", &log_prefix);

    Ok(rpc::handle_async_broker_stream(
        stream,
        agent_handler.clone(),
        nodes.clone(),
        term_receiver.clone(),
    )
    .await?)
}
