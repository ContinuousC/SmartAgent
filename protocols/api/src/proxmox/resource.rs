/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    collections::HashMap, fmt::Display, mem, net::Ipv4Addr, sync::Arc,
    time::SystemTime,
};

use chrono::{DateTime, Duration};
use etc_base::{ProtoDataFieldId, ProtoRow};
use futures::{stream, StreamExt};
use log::{debug, warn};
use protocol::CounterDb;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tap::Pipe;
use tokio::fs::{metadata, File};
use value::{Data, DataError, EnumValue, Value};

use crate::input::{FieldSpec, ParameterType, ValueTypes};

use super::{Client, DTEResult, DTError};

#[async_trait::async_trait]
pub trait Resource: DeserializeOwned + Send + Sync {
    const ENDPOINT: &'static str;
    const NODERESOURCE: bool = false;

    async fn from_client<'a>(client: Arc<Client<'a>>) -> DTEResult<Vec<Self>> {
        if Self::NODERESOURCE {
            client.request_noderesources().await
        } else {
            client.request_resource().await.map(|r| vec![r])
        }
    }

    fn into_data(
        self,
        datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<CounterDb>,
    ) -> ProtoRow;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    version: String,
    repoid: String,
    release: String,
}

#[async_trait::async_trait]
impl Resource for Version {
    const ENDPOINT: &'static str = "version";

    fn into_data(
        mut self,
        datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<CounterDb>,
    ) -> ProtoRow {
        datafields
            .iter()
            .map(|(&dfid, &df)| {
                (
                    dfid.clone(),
                    match df.parameter_header.as_str() {
                        "version" => Ok(Value::UnicodeString(mem::take(
                            &mut self.version,
                        ))),
                        "repoid" => Ok(Value::UnicodeString(mem::take(
                            &mut self.repoid,
                        ))),
                        "release" => Ok(Value::UnicodeString(mem::take(
                            &mut self.release,
                        ))),
                        _ => Err(DataError::Missing),
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClusterType {
    Node,
    Cluster,
}

impl ClusterType {
    fn as_enumvalue(&self, values: Option<&ValueTypes>) -> Data {
        match values
            .ok_or(DataError::TypeError("expected valuestypes".to_string()))?
        {
            ValueTypes::Integer(_) => {
                Err(DataError::TypeError("expected stringenum".to_string()))
            }
            ValueTypes::String(s) => {
                EnumValue::new(s.clone(), self.to_string()).map(Value::Enum)
            }
        }
    }
}

impl Display for ClusterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Node => "node",
                Self::Cluster => "cluster",
            }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    id: String,
    name: String,
    r#type: ClusterType,
    online: u8,
    local: u8,
    ip: Ipv4Addr,
    nodeid: i64,
    level: String,
}

#[async_trait::async_trait]
impl Resource for ClusterStatus {
    const ENDPOINT: &'static str = "cluster/status";

    async fn from_client<'a>(
        client: Arc<Client<'a>>,
    ) -> DTEResult<Vec<ClusterStatus>> {
        client.request_resourcelist().await
    }

    fn into_data(
        mut self,
        datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<CounterDb>,
    ) -> ProtoRow {
        datafields
            .iter()
            .map(|(&dfid, &df)| {
                (
                    dfid.clone(),
                    match df.parameter_header.as_str() {
                        "online" => Ok(Value::Boolean(self.online == 1)),
                        "local" => Ok(Value::Boolean(self.local == 1)),
                        "type" => self.r#type.as_enumvalue(df.values.as_ref()),
                        "nodeid" => Ok(Value::Integer(self.nodeid)),

                        "name" => {
                            Ok(Value::UnicodeString(mem::take(&mut self.name)))
                        }
                        "id" => {
                            Ok(Value::UnicodeString(mem::take(&mut self.id)))
                        }
                        "level" => {
                            Ok(Value::UnicodeString(mem::take(&mut self.level)))
                        }
                        "ip" => Ok(Value::Ipv4Address(self.ip.octets())),

                        _ => Err(DataError::Missing),
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QmStatus {
    Stopped,
    Running,
}

impl QmStatus {
    fn into_enumvalue(self, values: Option<&ValueTypes>) -> Data {
        match values
            .ok_or(DataError::TypeError("expected valuestypes".to_string()))?
        {
            ValueTypes::Integer(_) => {
                Err(DataError::TypeError("expected stringenum".to_string()))
            }
            ValueTypes::String(s) => {
                EnumValue::new(s.clone(), self.to_string()).map(Value::Enum)
            }
        }
    }
}

impl Display for QmStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Stopped => "stopped",
                Self::Running => "running",
            }
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmStatus {
    pub vmid: u64,
    name: String,
    status: QmStatus,
    uptime: u64,
    // pid: u16,
    // agent: u8,
    cpu: f64,
    cpus: u16,

    diskread: u64,
    diskwrite: u64,
    maxdisk: i64,

    mem: i64,
    maxmem: i64,
    // freemem: i64,
    netin: u64,
    netout: u64,
}

#[async_trait::async_trait]
impl Resource for VmStatus {
    const ENDPOINT: &'static str = "qemu";
    const NODERESOURCE: bool = true;

    async fn from_client<'a>(client: Arc<Client<'a>>) -> DTEResult<Vec<Self>> {
        client.get_qemus().await.map(|qemus| qemus.to_vec())
    }

    fn into_data(
        mut self,
        datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<CounterDb>,
    ) -> ProtoRow {
        let calculate_u64counter = |parameter_type, key, value| {
            let key = format!("{}.{}", self.vmid, key);
            match parameter_type {
                ParameterType::Counter => {
                    counterdb.counter(key, value, SystemTime::now())
                }
                ParameterType::Difference => {
                    counterdb.difference(key, value, SystemTime::now())
                }
                _ => {
                    Err(DataError::TypeError("expected a counter".to_string()))
                }
            }
        };

        datafields
            .iter()
            .map(|(&dfid, &df)| {
                (
                    dfid.clone(),
                    match df.parameter_header.as_str() {
                        "vmid" => Ok(Value::Integer(self.vmid as i64)),
                        "name" => {
                            Ok(Value::UnicodeString(mem::take(&mut self.name)))
                        }
                        "status" => {
                            self.status.into_enumvalue(df.values.as_ref())
                        }
                        "uptime" => Ok(Value::Age(Duration::seconds(
                            self.uptime as i64,
                        ))),
                        // "pid" => Ok(Value::Integer(self.pid as i64)),
                        // "agent" => Ok(Value::Boolean(self.agent == 1)),
                        "cpu" => Ok(Value::Float(self.cpu)),
                        "cpus" => Ok(Value::Integer(self.cpus as i64)),

                        "diskread" => calculate_u64counter(
                            df.parameter_type,
                            &df.parameter_name,
                            self.diskread,
                        ),
                        "diskwrite" => calculate_u64counter(
                            df.parameter_type,
                            &df.parameter_name,
                            self.diskwrite,
                        ),
                        "maxdisk" => Ok(Value::Integer(self.maxdisk)),

                        "mem" => Ok(Value::Integer(self.mem)),
                        "maxmem" => Ok(Value::Integer(self.maxmem)),
                        // "freemem" => Ok(Value::Integer(self.freemem)),
                        "netin" => calculate_u64counter(
                            df.parameter_type,
                            &df.parameter_name,
                            self.netin,
                        ),
                        "netout" => calculate_u64counter(
                            df.parameter_type,
                            &df.parameter_name,
                            self.netout,
                        ),

                        _ => Err(DataError::Missing),
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmSnapshot {
    #[serde(default)]
    vmid: u64,
    description: String,
    name: String,
    #[serde(default)]
    snaptime: u64,
    #[serde(default)]
    vmstate: u8,
}

#[async_trait::async_trait]
impl Resource for VmSnapshot {
    const ENDPOINT: &'static str = "qemu/{qemuid}/snapshot";
    const NODERESOURCE: bool = true;

    async fn from_client<'a>(client: Arc<Client<'a>>) -> DTEResult<Vec<Self>> {
        let vmids = client.get_qemuids().await?;
        let vms: Vec<DTEResult<Vec<VmSnapshot>>> = vmids
            .iter()
            .copied()
            .map(|vmid| {
                client.request_list(format!(
                    "{}/nodes/{}/qemu/{vmid}/snapshot",
                    client.base_url, client.node
                ))
            })
            .pipe(stream::iter)
            .buffered(vmids.len())
            .collect()
            .await;

        let mut snapshots = Vec::with_capacity(vms.len());
        for (vmid, snapshot) in vmids.iter().zip(vms) {
            let mut snapshot = snapshot?
                .into_iter()
                .max_by_key(|snapshot| snapshot.snaptime)
                .unwrap();

            snapshot.vmid = *vmid;
            snapshots.push(snapshot)
        }

        Ok(snapshots)
    }

    fn into_data(
        mut self,
        datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<CounterDb>,
    ) -> ProtoRow {
        datafields
            .iter()
            .map(|(&dfid, &df)| {
                (
                    dfid.clone(),
                    match df.parameter_header.as_str() {
                        "vmid" => Ok(Value::Integer(self.vmid as i64)),
                        "description" => Ok(Value::UnicodeString(mem::take(
                            &mut self.description,
                        ))),
                        "name" => {
                            Ok(Value::UnicodeString(mem::take(&mut self.name)))
                        }
                        "vmstate" => Ok(Value::Boolean(self.vmstate == 1)),
                        "snaptime" => {
                            if self.snaptime == 0 {
                                Err(DataError::External(
                                    "no snapshot taken yet".to_string(),
                                ))
                            } else {
                                (SystemTime::UNIX_EPOCH
                                    + std::time::Duration::from_secs(
                                        self.snaptime,
                                    ))
                                .elapsed()
                                .unwrap()
                                .pipe(
                                    |d| {
                                        Ok(Value::Age(
                                            Duration::from_std(d).unwrap(),
                                        ))
                                    },
                                )
                            }
                        }

                        _ => Err(DataError::Missing),
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LxcStatus {
    pub vmid: u64,
    name: String,
    status: QmStatus,
    uptime: u64,

    cpu: f64,
    cpus: u16,

    diskread: u64,
    diskwrite: u64,
    maxdisk: i64,

    mem: i64,
    maxmem: i64,
    // freemem: i64,
    netin: u64,
    netout: u64,
}

#[async_trait::async_trait]
impl Resource for LxcStatus {
    const ENDPOINT: &'static str = "lxc";
    const NODERESOURCE: bool = true;

    async fn from_client<'a>(client: Arc<Client<'a>>) -> DTEResult<Vec<Self>> {
        client.get_lcxs().await.map(|lcxs| lcxs.to_vec())
    }

    fn into_data(
        mut self,
        datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        counterdb: Arc<CounterDb>,
    ) -> ProtoRow {
        let calculate_u64counter = |parameter_type, key, value| {
            let key = format!("{}.{}", self.vmid, key);
            match parameter_type {
                ParameterType::Counter => {
                    counterdb.counter(key, value, SystemTime::now())
                }
                ParameterType::Difference => {
                    counterdb.difference(key, value, SystemTime::now())
                }
                _ => {
                    Err(DataError::TypeError("expected a counter".to_string()))
                }
            }
        };

        datafields
            .iter()
            .map(|(&dfid, &df)| {
                (
                    dfid.clone(),
                    match df.parameter_header.as_str() {
                        "vmid" => Ok(Value::Integer(self.vmid as i64)),
                        "name" => {
                            Ok(Value::UnicodeString(mem::take(&mut self.name)))
                        }
                        "status" => {
                            self.status.into_enumvalue(df.values.as_ref())
                        }
                        "uptime" => Ok(Value::Age(Duration::seconds(
                            self.uptime as i64,
                        ))),

                        "cpu" => Ok(Value::Float(self.cpu)),
                        "cpus" => Ok(Value::Integer(self.cpus as i64)),

                        "diskread" => calculate_u64counter(
                            df.parameter_type,
                            &df.parameter_name,
                            self.diskread,
                        ),
                        "diskwrite" => calculate_u64counter(
                            df.parameter_type,
                            &df.parameter_name,
                            self.diskwrite,
                        ),
                        "maxdisk" => Ok(Value::Integer(self.maxdisk)),

                        "mem" => Ok(Value::Integer(self.mem)),
                        "maxmem" => Ok(Value::Integer(self.maxmem)),
                        // "freemem" => Ok(Value::Integer(self.freemem)),
                        "netin" => calculate_u64counter(
                            df.parameter_type,
                            &df.parameter_name,
                            self.netin,
                        ),
                        "netout" => calculate_u64counter(
                            df.parameter_type,
                            &df.parameter_name,
                            self.netout,
                        ),

                        _ => Err(DataError::Missing),
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    id: String,
    // name: String,
    pid: i64,
    upid: String,
    starttime: i64,
    endtime: i64,
    status: String,
    r#type: String,
    user: String,
}

#[async_trait::async_trait]
impl Resource for Task {
    const ENDPOINT: &'static str = "tasks";
    const NODERESOURCE: bool = true;

    async fn from_client<'a>(client: Arc<Client<'a>>) -> DTEResult<Vec<Self>> {
        let tspath = client.plugin.cache_dir.join("tasks.timestamp");
        let seconds_ago = match tspath.exists() {
            true => metadata(&tspath)
                .await
                .map_err(|e| DTError::FileAccess(e, tspath.clone()))?
                .modified()
                .unwrap()
                .elapsed()
                .unwrap(),
            false => std::time::Duration::from_secs(60 * 60 * 24 * 7),
        };
        let since = SystemTime::UNIX_EPOCH.elapsed().unwrap().as_secs()
            - seconds_ago.as_secs();
        debug!(
            "requesting tasks since {since} ({}s ago)",
            seconds_ago.as_secs_f32()
        );

        let url =
            format!("{}?since={since}", client.node_resource(Self::ENDPOINT),);
        let data = client.request_list(url).await?;

        if let Err(e) = File::options()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tspath)
            .await
        {
            warn!(
                "failed to update/create timestampfile {}: {e}",
                tspath.display()
            );
        }

        Ok(data)
    }

    fn into_data(
        mut self,
        datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<CounterDb>,
    ) -> ProtoRow {
        datafields
            .iter()
            .map(|(&dfid, &df)| {
                (
                    dfid.clone(),
                    match df.parameter_header.as_str() {
                        "endtime" => Ok(Value::Time(
                            DateTime::from_timestamp(self.endtime, 0).unwrap(),
                        )),
                        "id" => {
                            Ok(Value::UnicodeString(mem::take(&mut self.id)))
                        }
                        // "name" => {
                        //     Ok(Value::UnicodeString(mem::take(&mut self.name)))
                        // }
                        "pid" => Ok(Value::Integer(self.pid)),
                        "starttime" => Ok(Value::Time(
                            DateTime::from_timestamp(self.starttime, 0)
                                .unwrap(),
                        )),
                        "status" => Ok(Value::UnicodeString(mem::take(
                            &mut self.status,
                        ))),
                        "type" => Ok(Value::UnicodeString(mem::take(
                            &mut self.r#type,
                        ))),
                        "upid" => {
                            Ok(Value::UnicodeString(mem::take(&mut self.upid)))
                        }
                        "user" => {
                            Ok(Value::UnicodeString(mem::take(&mut self.user)))
                        }

                        _ => Err(DataError::Missing),
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Storage {
    storage: String,
    r#type: String,

    active: u8,
    content: String,
    enabled: u8,
    shared: u8,

    avail: i64,
    total: i64,
    used: i64,
}

impl Resource for Storage {
    const ENDPOINT: &'static str = "storage";
    const NODERESOURCE: bool = true;

    fn into_data(
        mut self,
        datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<CounterDb>,
    ) -> ProtoRow {
        datafields
            .iter()
            .map(|(&dfid, &df)| {
                (
                    dfid.clone(),
                    match df.parameter_header.as_str() {
                        "active" => Ok(Value::Boolean(self.active == 1)),
                        "avail" => Ok(Value::Integer(self.avail)),
                        "content" => Ok(Value::UnicodeString(mem::take(
                            &mut self.content,
                        ))),
                        "enabled" => Ok(Value::Boolean(self.enabled == 1)),
                        "shared" => Ok(Value::Boolean(self.shared == 1)),
                        "storage" => Ok(Value::UnicodeString(mem::take(
                            &mut self.storage,
                        ))),
                        "total" => Ok(Value::Integer(self.total)),
                        "type" => Ok(Value::UnicodeString(mem::take(
                            &mut self.r#type,
                        ))),
                        "used" => Ok(Value::Integer(self.used)),

                        _ => Err(DataError::Missing),
                    },
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replication {}

impl Resource for Replication {
    const ENDPOINT: &'static str = "replication";
    const NODERESOURCE: bool = true;

    fn into_data(
        self,
        _datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<CounterDb>,
    ) -> ProtoRow {
        todo!()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CephStatus {}

impl Resource for CephStatus {
    const ENDPOINT: &'static str = "ceph/status";
    const NODERESOURCE: bool = true;

    fn into_data(
        self,
        _datafields: &HashMap<&ProtoDataFieldId, &FieldSpec>,
        _counterdb: Arc<CounterDb>,
    ) -> ProtoRow {
        todo!()
    }
}
