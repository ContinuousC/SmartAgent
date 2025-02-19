/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

pub mod config;
pub mod error;
#[macro_use]
pub mod context;
mod broker_connection;

use std::pin::Pin;
use std::sync::Arc;
use std::{path::PathBuf, process};

use agent_utils::KeyVault;
use clap::{App, Arg};
use dbschema::Timestamped;
use futures::Future;
use metrics_types::{Data, MetricsTable};
use tokio::{
    signal::unix::{signal, SignalKind},
    sync::{mpsc, watch},
};

use broker_api::{AgentToBrokerMessage, AgentToBrokerMessageCompat};
use metrics_engine_api::{AgentMetricsProto, AgentMetricsServiceStub};
use rpc::{
    AsyncBrokerClient, AsyncDuplex, AsyncRequest, AsyncResponse,
    AsyncServerConnection, Connector, MsgReadStream, MsgWriteStream,
};

use agent_service::AgentService;
//use backend_connector::{BackendConnector, BackendConnectorEvent};
use etc::EtcManager;
use protocol::PluginManager;
use scheduler::Scheduler;

use error::{Error, Result};

#[tokio::main]
async fn main() {
    let matches = App::new("SmartM Agent")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Maarten Deprez <mdp@si-int.eu>")
        .author("Vincent Stuyck <vst@si-int.eu>")
        .about("Periodically retrieves data from remote systems.")
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .multiple(true)
                .help(
                    "Increase verbosity. This option can be specified multiple times. \
					 The maximum verbosity level is 3.",
                ),
        )
        .arg(Arg::with_name("log-allow-module")
			 .long("log-allow-module")
			 .takes_value(true)
			 .multiple(true)
			 .help("Only log output from specific module(s)")
		)
        .arg(Arg::with_name("log-ignore-module")
			 .long("log-ignore-module")
			 .takes_value(true)
			 .multiple(true)
			 .help("Ignore log output from specific module(s)")
		)
        .arg(
            Arg::with_name("connect")
                .long("connect")
                .takes_value(true)
                .help("Connect to the broker on this address."),
        )
        .arg(
            Arg::with_name("listen")
                .long("listen")
                .takes_value(true)
                .help("Listen for broker connections on this address."),
        )
        .arg(
            Arg::with_name("broker")
                .long("broker")
                .takes_value(true)
                .required(true)
                .help("The domain name of the broker (for certificate validation)."),
        )
		.arg(
            Arg::with_name("broker-compat")
                .long("broker-compat")
                .help("Communicate with older broker"),
        )
        .arg(
            Arg::with_name("ca-cert")
                .long("ca-cert")
                .takes_value(true)
                .help("The certificate of the CA."),
        )
        .arg(
            Arg::with_name("cert")
                .long("cert")
                .takes_value(true)
                .help("The certificate of the agent."),
        )
        .arg(
            Arg::with_name("key")
                .long("key")
                .takes_value(true)
                .help("The private key of the agent."),
        )
        .get_matches();

    let mut log_config = simplelog::ConfigBuilder::new();

    if let Some(vals) = matches.values_of("log-allow-module") {
        for module in vals {
            log_config.add_filter_allow(module.to_string());
        }
    }

    if let Some(vals) = matches.values_of("log-ignore-module") {
        for module in vals {
            log_config.add_filter_ignore(module.to_string());
        }
    }

    if let Err(e) = simplelog::TermLogger::init(
        match matches.occurrences_of("verbose") {
            0 => simplelog::LevelFilter::Off,
            1 => simplelog::LevelFilter::Error,
            2 => simplelog::LevelFilter::Warn,
            3 => simplelog::LevelFilter::Info,
            4 => simplelog::LevelFilter::Debug,
            5.. => simplelog::LevelFilter::Trace,
        },
        log_config.build(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    ) {
        eprintln!("Error: failed to initialize logging: {}", e);
        process::exit(1);
    }

    let (listen, addr) = match matches.value_of("listen") {
        Some(addr) => (true, addr),
        None => match matches.value_of("connect") {
            Some(addr) => (false, addr),
            None => (false, "[::1]:9999"),
        },
    };
    let broker_domain = matches.value_of("broker").unwrap();
    let ca_path =
        PathBuf::from(matches.value_of("ca-cert").unwrap_or("certs/ca.crt"));
    let cert_path =
        PathBuf::from(matches.value_of("cert").unwrap_or("certs/agent.crt"));
    let key_path =
        PathBuf::from(matches.value_of("key").unwrap_or("certs/agent.key"));

    let tls_config = rpc::tls_client_config(&ca_path, &cert_path, &key_path)
        .await
        .expect("failed to create tls client config");

    let (data_sender, data_receiver) = mpsc::channel(100);

    let vault = KeyVault::Identity;
    let mut plugin_manager = PluginManager::new();
    let cache_path = PathBuf::from("/tmp/smart-agent");
    plugin_manager.add_plugin(snmp_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));
    plugin_manager.add_plugin(azure_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));
    plugin_manager.add_plugin(wmi_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));
    plugin_manager.add_plugin(api_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));
    plugin_manager.add_plugin(ssh_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
        PathBuf::new(),
        matches.occurrences_of("verbose") as u8,
    ));
    plugin_manager.add_plugin(powershell_protocol::Plugin::new(
        cache_path.clone(),
        vault.clone(),
    ));

    let plugin_manager = Arc::new(plugin_manager);
    let etc_manager = Arc::new(EtcManager::new());
    let scheduler = Scheduler::new(
        plugin_manager.clone(),
        etc_manager.spec_receiver().await,
        data_sender,
    );
    let agent_service = Arc::new(
        AgentService::new(plugin_manager, etc_manager, scheduler).unwrap(),
    );

    let (agent_req_sender, agent_req_receiver) = mpsc::channel(1000);
    let (metrics_engine_res_sender, metrics_engine_res_receiver) =
        mpsc::channel(1000);

    let broker_compat = matches.is_present("broker-compat");

    let (agent_res_sender, metrics_engine_req_sender, broker_shutdown): (
        Box<dyn MsgWriteStream<((), AsyncResponse<serde_cbor::Value>)>>,
        Box<dyn MsgWriteStream<((), AsyncRequest<serde_cbor::Value>)>>,
        Box<dyn FnOnce() -> Pin<Box<dyn Future<Output = rpc::Result<()>>>>>,
    ) = match broker_compat {
        false => {
            let broker_handler =
                broker_connection::broker_message_handler::<serde_cbor::Value>(
                    agent_req_sender,
                    metrics_engine_res_sender.clone(),
                );
            let broker_unconnected =
                broker_connection::broker_unconnected_handler(
                    metrics_engine_res_sender,
                );

            let broker = match listen {
                true => {
                    let connector = Connector::tcp_listener(addr.to_string())
                        .await
                        .expect("failed to listen for broker connections")
                        .tls(tls_config, broker_domain)
                        .expect("failed to initialize TLS")
                        .cbor();
                    AsyncBrokerClient::new_unconnected(
                        connector,
                        broker_handler,
                        broker_unconnected,
                    )
                    .expect("failed to init broker connection")
                }
                false => {
                    let connector = Connector::tcp(addr.to_string())
                        .await
                        .tls(tls_config, broker_domain)
                        .expect("failed to initialize TLS")
                        .cbor();
                    AsyncBrokerClient::new_unconnected(
                        connector,
                        broker_handler,
                        broker_unconnected,
                    )
                    .expect("failed to init broker connection")
                }
            };

            (
                Box::new(broker.sender().map(|((), message)| {
                    AgentToBrokerMessage::Backend { message }
                })),
                Box::new(broker.sender().map(|((), message)| {
                    AgentToBrokerMessage::MetricsEngine { message }
                })),
                Box::new(move || Box::pin(broker.shutdown())),
            )
        }
        // broker_compat = true
        true => {
            let broker_handler =
                broker_connection::broker_message_handler_compat::<
                    serde_cbor::Value,
                >(
                    agent_req_sender, metrics_engine_res_sender.clone()
                );
            let broker_unconnected =
                broker_connection::broker_unconnected_handler_compat(
                    metrics_engine_res_sender,
                );

            let broker = match listen {
                true => {
                    let connector = Connector::tcp_listener(addr.to_string())
                        .await
                        .expect("failed to listen for broker connections")
                        .tls(tls_config, broker_domain)
                        .expect("failed to initialize TLS")
                        .cbor_compat(0);
                    AsyncBrokerClient::new_unconnected(
                        connector,
                        broker_handler,
                        broker_unconnected,
                    )
                    .expect("failed to init broker connection")
                }
                false => {
                    let connector = Connector::tcp(addr.to_string())
                        .await
                        .tls(tls_config, broker_domain)
                        .expect("failed to initialize TLS")
                        .cbor_compat(0);
                    AsyncBrokerClient::new_unconnected(
                        connector,
                        broker_handler,
                        broker_unconnected,
                    )
                    .expect("failed to init broker connection")
                }
            };

            (
                Box::new(broker.sender().map(|((), message)| {
                    AgentToBrokerMessageCompat::Backend {
                        message: AsyncDuplex::Response(message),
                    }
                })),
                Box::new(broker.sender().map(|((), message)| {
                    AgentToBrokerMessageCompat::Database {
                        message: AsyncDuplex::Request(message),
                    }
                })),
                Box::new(move || Box::pin(broker.shutdown())),
            )
        }
    };

    let _agent = AsyncServerConnection::new_split(
        agent_req_receiver.map(|msg| ((), msg)),
        agent_res_sender,
        Arc::new(agent_api::AgentHandler::new(agent_service.clone())),
    );

    let metrics_engine =
        AgentMetricsServiceStub::new(rpc::AsyncClientConnection::new_split(
            metrics_engine_res_receiver.map(|msg| ((), msg)),
            metrics_engine_req_sender,
        ));

    let (term_sender, term_receiver) = watch::channel(false);

    let data_writer =
        tokio::spawn(data_writer(data_receiver, metrics_engine, term_receiver));

    let mut sigint = signal(SignalKind::interrupt())
        .expect("failed to install sigint handler");
    let mut sigterm = signal(SignalKind::terminate())
        .expect("failed to install sigterm handler");

    tokio::select! {
        _ = sigint.recv() => {}
        _ = sigterm.recv() => {}
    };

    eprintln!("Awaiting open connections (press ctrl-c to force shutdown)...");

    let mut shutdown = Box::pin(async {
        if let Err(e) = term_sender.send(true) {
            eprintln!("Warning: failed to send termination signal: {}", e);
        }
        // if let Err(e) = agent.shutdown().await {
        //     eprintln!("Warning: failed to shut down agent service: {}", e);
        // }
        // if let Ok(srv) = Arc::try_unwrap(agent_service) {
        //     let _ = srv.scheduler.shutdown().await;
        // }
        match data_writer.await {
            Err(e) => {
                eprintln!("Warning: failed to join data writer: {}", e);
            }
            Ok(Err(e)) => {
                eprintln!("Warning: data writer failed: {}", e)
            }
            Ok(Ok(())) => {}
        }
        if let Err(e) = broker_shutdown().await {
            eprintln!("Warning: failed to shut down broker connection: {}", e);
        }
    });

    tokio::select! {
        _ = &mut shutdown => {}
        _ = sigint.recv() => {
            eprintln!("Received SIGINT; force shutdown!");
        }
        _ = sigterm.recv() => {
            eprintln!("Received SIGTERM; force shutdown!");
        }
    }
}

async fn data_writer(
    mut receiver: mpsc::Receiver<(
        String,
        String,
        Timestamped<MetricsTable<Data<serde_json::Value>>>,
    )>,
    metrics_engine: AgentMetricsServiceStub<
        rpc::AsyncClientConnection<AgentMetricsProto, serde_cbor::Value, ()>,
        serde_cbor::Value,
    >,
    mut term_receiver: watch::Receiver<bool>,
) -> Result<()> {
    while !*term_receiver.borrow() {
        let (mp, table, data) = tokio::select! {
            data = receiver.recv() => {
                match data {
                    Some(data) => data,
                    None => break
                }
            },
            _ = term_receiver.changed() => continue
        };
        let res = tokio::select! {
            r = metrics_engine.create_metrics(mp, table, data) => r,
            _ = tokio::time::sleep(std::time::Duration::from_secs(10))
                => Err(Error::Timeout.to_string())
        };
        if let Err(e) = res {
            eprintln!("Warning: failed to write data: {}", e);
        }
    }
    metrics_engine.into_inner().shutdown().await?;
    Ok(())
}
