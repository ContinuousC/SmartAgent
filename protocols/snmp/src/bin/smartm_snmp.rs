/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

// use std::path::PathBuf;

// use agent_utils::KeyVault;
// use clap::Parser;
// use protocol::{ProtocolHandler, ProtocolProto};
// use protocol_daemon::ProtocolDaemon;
// use tokio::signal::unix::{signal, SignalKind};

// #[derive(Parser)]
// #[clap(version, author)]
// struct Args {
//     /// Socket path.
//     #[clap(
//         long = "socket",
//         short = 's',
//         default_value = "/var/lib/smartm/snmp.sock"
//     )]
//     sock_path: PathBuf,
//     /// Cache directory.
//     #[clap(long = "cache", short = 'c', default_value = "/tmp/smartm/snmp")]
//     cache_path: PathBuf,
//     /// Increase logging verbosity.
//     #[clap(long = "verbose", short = 'v', parse(from_occurrences))]
//     verbose: u8,
// }

// impl Args {
//     fn verbosity(&self) -> simplelog::LevelFilter {
//         match self.verbose {
//             0 => simplelog::LevelFilter::Warn,
//             1 => simplelog::LevelFilter::Info,
//             2 => simplelog::LevelFilter::Debug,
//             3.. => simplelog::LevelFilter::Trace,
//         }
//     }
// }

// #[tokio::main]
// async fn main() {
//     let args = Args::parse();

//     if let Err(e) = simplelog::TermLogger::init(
//         args.verbosity(),
//         simplelog::Config::default(),
//         simplelog::TerminalMode::Stderr,
//         simplelog::ColorChoice::Auto,
//     ) {
//         log::error!("failed to initialize logging: {}", e);
//         std::process::exit(1);
//     }

//     if let Some(parent) = args.sock_path.parent() {
//         tokio::fs::create_dir_all(parent).await.unwrap();
//     }

//     if let Some(parent) = args.cache_path.parent() {
//         tokio::fs::create_dir_all(parent).await.unwrap();
//     }

//     let key_vault = KeyVault::Identity;

//     let snmp = snmp_protocol::Plugin::new(args.cache_path.clone(), key_vault);
//     let service = ProtocolDaemon::new(snmp);
//     let server = rpc::AsyncServer::<ProtocolProto>::builder()
//         .unix(args.sock_path.clone())
//         .unwrap()
//         .notls()
//         .json()
//         .handler_session(ProtocolHandler::new(service));

//     let mut sigint = signal(SignalKind::interrupt()).unwrap();
//     let mut sigterm = signal(SignalKind::terminate()).unwrap();

//     log::info!("Listening for requests on {}", args.sock_path.display());

//     tokio::select! {
//         _ = sigint.recv() => {}
//         _ = sigterm.recv() => {}
//     };

//     log::info!("Awaiting open connections (press ctrl-c to force shutdown)...");

//     tokio::select! {
//             r = server.shutdown() => {
//             r.unwrap();
//             log::info!("Server shut down successfully.");
//         }
//         _ = sigint.recv() => {
//             log::info!("Received SIGINT; force shutdown!");
//         }
//         _ = sigterm.recv() => {
//             log::info!("Received SIGTERM; force shutdown!");
//         }
//     };

//     tokio::fs::remove_file(args.sock_path).await.unwrap();
// }

fn main() {}
