/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

mod agent_handler;
mod backend_handler;
mod broker_service;
mod database_handler;
mod error;
mod node;
mod ssh_connector;

use std::collections::HashMap;
use std::process;
use std::sync::RwLock;
use std::{path::PathBuf, sync::Arc};

use clap::{App, Arg};
use serde_cbor::Value;
use tokio::signal::unix::{signal, SignalKind};

use agent_handler::AgentHandler;
use backend_handler::BackendHandler;
use database_handler::DatabaseHandler;
use error::{Error, Result};
use node::Node;

use crate::broker_service::BrokerService;

#[tokio::main]
async fn main() {
    // console_subscriber::init();

    let matches = App::new("ContinuousC Broker")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Maarten Deprez <mdp@si-int.eu>")
        .about("Connect agents with the backend and the metrics engine")
        .arg(
            Arg::with_name("agent")
                .long("agent")
                .takes_value(true)
                .help("The address on which to listen for incoming agent connections."),
        )
        .arg(
            Arg::with_name("backend")
                .long("backend")
                .takes_value(true)
                .help("The address on which to listen for incoming backend connections."),
        )
        .arg(
            Arg::with_name("dbdaemon")
                .long("dbdaemon")
                .takes_value(true)
                .help("The address on which to listen for incoming db daemon connections."),
        )
		.arg(
            Arg::with_name("certs-dir")
                .long("certs-dir")
                .help("The path in which to look for certificates")
                .default_value("/usr/share/continuousc/certs/broker"),
        )
		.arg(
            Arg::with_name("ca-cert")
                .long("ca-cert")
				.takes_value(true)
				.default_value("ca.crt")
                .help("The certificate of the CA."),
        )
        .arg(
            Arg::with_name("cert")
                .long("cert")
                .takes_value(true)
				.default_value("broker.crt")
                .help("The certificate of the broker."),
        )
        .arg(
            Arg::with_name("key")
                .long("key")
                .takes_value(true)
				.default_value("broker.key")
                .help("The private key of the broker."),
        )
        .arg(Arg::with_name("verbose").long("verbose").short("v").multiple(true).help("Show informational messages."))
        .get_matches();

    if let Err(e) = simplelog::TermLogger::init(
        match matches.occurrences_of("verbose") {
            0 => simplelog::LevelFilter::Warn,
            1 => simplelog::LevelFilter::Info,
            2 => simplelog::LevelFilter::Debug,
            3.. => simplelog::LevelFilter::Trace,
        },
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    ) {
        eprintln!("Error: failed to initialize logging: {}", e);
        process::exit(1);
    }

    let certs_dir = PathBuf::from(matches.value_of("certs-dir").unwrap());
    if let Err(e) = run(
        certs_dir.join(matches.value_of("ca-cert").unwrap()),
        certs_dir.join(matches.value_of("cert").unwrap()),
        certs_dir.join(matches.value_of("key").unwrap()),
        matches.value_of("agent").unwrap_or("[::]:9999").to_string(),
        matches
            .value_of("backend")
            .unwrap_or("[::]:9998")
            .to_string(),
        matches
            .value_of("dbdaemon")
            .unwrap_or("[::]:9997")
            .to_string(),
        "mndev02".to_string(), /* server name */
        9999,                  /* server_port */
    )
    .await
    {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run(
    ca_path: PathBuf,
    cert_path: PathBuf,
    key_path: PathBuf,
    agent_addr: String,
    backend_addr: String,
    db_addr: String,
    server_name: String,
    server_port: u32,
) -> Result<()> {
    let tls_config =
        rpc::tls_server_config(&ca_path, &cert_path, &key_path).await?;

    let node_map = Arc::new(RwLock::new(HashMap::new()));

    let broker_handler =
        Arc::new(broker_api::BrokerHandler::new(BrokerService::new(
            HashMap::new(),
            node_map.clone(),
            tls_config.clone(),
            server_name,
            server_port,
        )));

    let broker = rpc::AsyncBroker::<Node<Value>>::builder_with_nodes(node_map)
        .handler(
            rpc::AsyncBrokerHandlerBuilder::<Node<Value>, _>::new()
                .tcp(agent_addr)
                .await?
                .tls(tls_config.clone())
                .handler(AgentHandler::<Value>::new()),
        )
        .handler(
            rpc::AsyncBrokerHandlerBuilder::<Node<Value>, _>::new()
                .tcp(backend_addr)
                .await?
                .tls(tls_config.clone())
                .handler(BackendHandler::new(broker_handler)),
        )
        .handler(
            rpc::AsyncBrokerHandlerBuilder::<Node<Value>, _>::new()
                .tcp(db_addr)
                .await?
                .tls(tls_config.clone())
                .handler(DatabaseHandler::new()),
        )
        .build();

    // signal handling
    //let mut sighup = signal(SignalKind::hangup()).map_err(Error::SignalInit)?;
    let mut sigint =
        signal(SignalKind::interrupt()).map_err(Error::SignalInit)?;
    let mut sigterm =
        signal(SignalKind::terminate()).map_err(Error::SignalInit)?;

    tokio::select! {
        _ = sigint.recv() => {}
        _ = sigterm.recv() => {}
    };

    eprintln!("Awaiting open connections (press ctrl-c to force shutdown)...");

    tokio::select! {
        r = broker.shutdown() => { r? }
        _ = sigint.recv() => {
            eprintln!("Received SIGINT; force shutdown!");
        }
        _ = sigterm.recv() => {
            eprintln!("Received SIGTERM; force shutdown!");
        }
    };

    Ok(())
}
