/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

use super::error::{Error, Result};
use super::nmap_xml::{HostState, NmapRun};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PingStats {
    state: HostState,
    latency: Option<f64>,
}

pub async fn ping(hosts: Vec<String>) -> Result<HashMap<String, PingStats>> {
    let nmap = Command::new("nmap")
        .arg("-oX")
        .arg("-")
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
                        PingStats {
                            state: host.status.state,
                            latency: host
                                .times
                                .map(|ts| ts.srtt as f64 / 1000000.0),
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
