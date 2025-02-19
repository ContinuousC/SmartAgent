/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename = "nmaprun")]
pub struct NmapRun {
    #[serde(rename = "host", default)]
    pub hosts: Vec<Host>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Host {
    pub status: Status<HostState>,
    #[serde(rename = "address", default)]
    pub addresses: Vec<Address>,
    pub hostnames: HostNames,
    pub ports: Option<Ports>,
    pub times: Option<Times>,
    pub trace: Option<Trace>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Status<T> {
    pub state: T,
    pub reason: Reason,
    pub reason_ttl: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(tag = "addrtype", content = "addr")]
#[serde(rename_all = "lowercase")]
pub enum Address {
    IPv4(std::net::Ipv4Addr),
    IPv6(std::net::Ipv6Addr),
    MAC(String),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostNames {
    #[serde(rename = "hostname", default)]
    pub hostnames: Vec<HostName>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HostName {
    pub name: String,
    pub r#type: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Ports {
    #[serde(rename = "port", default)]
    pub ports: Vec<Port>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Port {
    pub protocol: Proto,
    pub portid: u16,
    pub state: Status<PortState>,
    pub service: Option<Service>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Times {
    pub srtt: i64,
    pub rttvar: u64,
    pub to: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Trace {
    #[serde(rename = "hop", default)]
    pub hops: Vec<Hop>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Hop {
    pub ttl: u8,
    pub rtt: f64,
    pub host: Option<String>,
    pub ipaddr: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum HostState {
    Up,
    Down,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum PortState {
    Open,
    #[serde(rename = "open|filtered")]
    OpenFiltered,
    Closed,
    Filtered,
    Unfiltered,
    #[serde(rename = "closed|filtered")]
    ClosedFiltered,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    SynAck,
    ArpResponse,
    EchoReply,
    LocalhostResponse,
    ConnRefused,
    NoResponse,
    PortUnreach,
    UdpResponse,
    Reset,
    HostUnreach,
    HostProhibited,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Proto {
    Tcp,
    Udp,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Service {
    pub name: String,
    pub method: String,
    pub conf: u16, /* ??? */
}

impl Host {
    pub fn name(&self, args: &[String]) -> String {
        for hostname in &self.hostnames.hostnames {
            if &hostname.r#type == "user" && args.contains(&hostname.name) {
                return hostname.name.to_string();
            }
        }

        let addresses: HashMap<Address, &String> = args
            .iter()
            .filter_map(|arg| match std::net::IpAddr::from_str(arg) {
                Ok(std::net::IpAddr::V4(addr)) => {
                    Some((Address::IPv4(addr), arg))
                }
                Ok(std::net::IpAddr::V6(addr)) => {
                    Some((Address::IPv6(addr), arg))
                }
                Err(_) => None,
            })
            .collect();

        for address in &self.addresses {
            if let Some(arg) = addresses.get(address) {
                return arg.to_string();
            }
        }

        for address in &self.addresses {
            match address {
                Address::IPv4(addr) => return format!("{}", addr),
                Address::IPv6(addr) => return format!("{}", addr),
                _ => {}
            }
        }

        String::from("(unknown)")
    }
}
