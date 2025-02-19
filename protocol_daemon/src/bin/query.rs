/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use etc_base::{ProtoDataFieldId, ProtoDataTableId, Protocol};
use futures::stream::StreamExt;
use protocol::{ProtocolProto, ProtocolServiceStub};
use serde_json::value::RawValue;
// use rustls::ServerName;

#[derive(Parser)]
struct Args {
    /// The protocol daemon socket to connect to.
    #[clap(long, short)]
    daemon: PathBuf,
    /// The path in which to look for certificates.
    #[clap(long = "certs-dir", default_value = "/usr/share/continuousc/certs")]
    certs_dir: PathBuf,
    /// The certificate of the root ca.
    #[clap(long = "ca", default_value = "ca.crt")]
    ca: PathBuf,
    /// The client certificate chain.
    #[clap(long = "cert", default_value = "protocol-daemon-client.crt")]
    cert: PathBuf,
    /// The client private key.
    #[clap(long = "key", default_value = "protocol-daemon-client.key")]
    key: PathBuf,
    #[clap(long, short)]
    package: Vec<PathBuf>,
    #[clap(long, short)]
    config: PathBuf,
    #[clap(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    DataTable(QueryDataTable),
    // Table(QueryTable),
}

#[derive(clap::Parser)]
struct QueryDataTable {
    protocol: String,
    data_table_id: String,
    data_field_id: Vec<String>,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // let tls_client_config = rpc::tls_client_config(
    //     &args.certs_dir.join(&args.ca),
    //     &args.certs_dir.join(&args.cert),
    //     &args.certs_dir.join(&args.key),
    // )
    // .await
    // .unwrap();

    let client = ProtocolServiceStub::new(
        rpc::AsyncClient::<ProtocolProto, serde_json::Value, ()>::builder()
            .unix(args.daemon.clone())
            // .tls(
            //     tls_client_config,
            //     ServerName::try_from("localhost").unwrap(),
            // )
            .notls()
            .json(),
    );

    eprintln!(
        "Connecting to protocol daemon at {}...",
        args.daemon.display()
    );

    client
        .inner()
        .connected(std::time::Duration::from_secs(10))
        .await
        .expect("Failed to connect");

    eprintln!("Connected!");

    eprintln!("Loading packages...");

    let pkgs = futures::stream::iter(&args.package)
        .then(|path| {
            let path = path.clone();
            async {
                let data = tokio::fs::read_to_string(path)
                    .await
                    .expect("Failed to read package");
                serde_json::from_str::<etc::Package>(&data)
                    .expect("Failed to decode package.")
            }
        })
        .collect::<Vec<_>>()
        .await;

    let config = serde_json::from_str::<HashMap<String, Box<RawValue>>>(
        &tokio::fs::read_to_string(&args.config)
            .await
            .expect("Failed to read config"),
    )
    .expect("Failed to decode config");

    match &args.command {
        Command::DataTable(cmd) => {
            let proto = Protocol(cmd.protocol.to_string());

            let inputs = pkgs
                .into_iter()
                .filter_map(|mut pkg| {
                    Some(
                        pkg.input
                            .remove(&proto.clone())
                            .expect("Missing protocol in input"),
                    )
                })
                .collect::<Vec<_>>();
            let input = client
                .load_inputs(inputs)
                .await
                .expect("Failed to load packages");
            eprintln!("Inputs successfully loaded into protocol daemon.");

            let config = client
                .load_config(
                    config
                        .get(&cmd.protocol.to_lowercase())
                        .expect("Missing config for protocol")
                        .clone(),
                )
                .await
                .expect("Failed to load config");
            eprintln!("Config successfully loaded into protocol daemon.");

            eprintln!("Running query...");
            let res = client
                .run_queries(
                    HashMap::from_iter([(
                        ProtoDataTableId(cmd.data_table_id.clone()),
                        cmd.data_field_id
                            .iter()
                            .map(|id| ProtoDataFieldId(id.clone()))
                            .collect(),
                    )]),
                    input,
                    config,
                )
                .await
                .expect("Query failed")
                .remove(&ProtoDataTableId(cmd.data_table_id.clone()))
                .expect("Did not receive data table in result.");
            eprintln!("Query done!");

            println!(
                "{}",
                serde_json::to_string_pretty(&res)
                    .expect("Failed to encode result.")
            );
        }
    }
}
