/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::Write;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::process::Command;

use super::error::{Error, Result};
use super::nmap_xml::{NmapRun, PortState, Proto};

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone, Debug)]
#[serde(into = "String")]
#[serde(try_from = "&str")]
pub struct Port {
    proto: Proto,
    port: u16,
}

impl From<Port> for String {
    fn from(val: Port) -> Self {
        format!(
            "{}/{}",
            val.port,
            match val.proto {
                Proto::Tcp => "tcp",
                Proto::Udp => "udp",
            }
        )
    }
}

impl TryFrom<&str> for Port {
    type Error = PortConvertError;
    fn try_from(input: &str) -> std::result::Result<Self, Self::Error> {
        match input.split_once('/') {
            Some((port, proto)) => {
                let port = port
                    .parse()
                    .map_err(PortConvertError::InvalidPortNumber)?;
                let proto = match proto {
                    "udp" => Ok(Proto::Udp),
                    "tcp" => Ok(Proto::Tcp),
                    _ => Err(PortConvertError::InvalidProtocol(
                        proto.to_string(),
                    )),
                }?;
                Ok(Self { proto, port })
            }
            None => Err(PortConvertError::InvalidFormat),
        }
    }
}

#[derive(Error, Debug)]
pub enum PortConvertError {
    #[error("Invalid format; expected: portnumber/proto")]
    InvalidFormat,
    #[error("Unknown protocol: {0}")]
    InvalidProtocol(String),
    #[error("Invalid port number: {0}")]
    InvalidPortNumber(std::num::ParseIntError),
}

pub async fn portscan(
    hosts: Vec<String>,
    ports: Vec<Port>,
) -> Result<HashMap<String, HashMap<Port, PortState>>> {
    let mut ports_arg = String::from("-p");
    for (i, port) in ports.iter().enumerate() {
        if i > 0 {
            write!(ports_arg, ",").unwrap();
        }
        match &port.proto {
            Proto::Tcp => write!(ports_arg, "T:{}", port.port).unwrap(),
            Proto::Udp => write!(ports_arg, "U:{}", port.port).unwrap(),
        }
    }

    let nmap = Command::new("nmap")
        .arg("-oX")
        .arg("-")
        .arg("-n")
        .arg("-sTU")
        .arg(ports_arg)
        .args(hosts.clone())
        .kill_on_drop(true)
        .output()
        .await?;

    match nmap.status.code() {
        Some(0) => {
            let scan: NmapRun =
                serde_xml_rs::from_reader(nmap.stdout.as_slice())?;
            Ok(scan
                .hosts
                .into_iter()
                .map(|host| {
                    (
                        host.name(&hosts),
                        host.ports
                            .into_iter()
                            .flat_map(|ports| {
                                ports.ports.into_iter().map(|port| {
                                    (
                                        Port {
                                            proto: port.protocol,
                                            port: port.portid,
                                        },
                                        port.state.state,
                                    )
                                })
                            })
                            .collect(),
                    )
                })
                .collect())
        }
        c => Err(Error::NonZeroExitStatus(
            c,
            String::from_utf8_lossy(&nmap.stderr).into_owned(),
        )),
    }
}
