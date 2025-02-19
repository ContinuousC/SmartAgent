/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use super::error::{Error, Result};
use super::nmap_xml::{Hop, HostState, NmapRun};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Traceroute {
    state: HostState,
    latency: Option<f64>,
    hops: Option<Vec<Hop>>,
}

pub async fn traceroute(
    hosts: Vec<String>,
) -> Result<HashMap<String, Traceroute>> {
    let nmap = Command::new("nmap")
        .arg("-oX")
        .arg("-")
        .arg("--traceroute")
        .arg("-n")
        .arg("-sn")
        .arg("-PE")
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
                        Traceroute {
                            state: host.status.state,
                            latency: host
                                .times
                                .map(|ts| ts.srtt as f64 / 1000000.0),
                            hops: host.trace.map(|t| t.hops),
                        },
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
