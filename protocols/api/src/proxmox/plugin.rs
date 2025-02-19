/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::Arc;

use agent_utils::{KeyVault, TryGetFrom};
use etc_base::{
    Annotated, ProtoDataFieldId, ProtoDataTableId, ProtoQueryMap, ProtoRow,
};
use futures::{stream, StreamExt};
use log::{debug, error, info, warn};
use protocol::CounterDb;
use tap::TapFallible;

use crate::error::{DTError as APIDTError, Result as APIResult};
use crate::input::{FieldSpec, TableSpec};
use crate::plugin::TableData;
use crate::{plugin::DataMap, APIPlugin, Input, Plugin as ProtPlugin};

use super::resource::{
    ClusterStatus, LxcStatus, Resource, Storage, Task, Version, VmSnapshot,
    VmStatus,
};
use super::{Client, Config, DTEResult, Error, Result};

struct Request<'a> {
    client: Arc<Client<'a>>,
    counterdb: Arc<CounterDb>,
    datatable: (&'a ProtoDataTableId, &'a TableSpec),
    datafields: HashMap<&'a ProtoDataFieldId, &'a FieldSpec>,
}

pub struct Plugin {
    pub(crate) key_vault: KeyVault,
    pub(crate) cache_dir: PathBuf,
    pub(crate) config: Config,
}

impl Debug for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        enum KvValues {
            Identity,
            KeyReader,
        }

        impl From<&KeyVault> for KvValues {
            fn from(value: &KeyVault) -> Self {
                match value {
                    KeyVault::Identity => Self::Identity,
                    KeyVault::KeyReader(_) => Self::KeyReader,
                }
            }
        }

        f.debug_struct("Plugin")
            .field("key_vault", &KvValues::from(&self.key_vault))
            .field("cache_dir", &self.cache_dir)
            .field("config", &self.config)
            .finish()
    }
}

impl Plugin {
    pub fn new(
        cache_dir: PathBuf,
        key_vault: KeyVault,
        config: Config,
    ) -> Self {
        Self {
            key_vault,
            cache_dir,
            config,
        }
    }

    async fn request_resource<T: Resource>(
        &self,
        request: Request<'_>,
    ) -> DTEResult<Vec<ProtoRow>> {
        let resources = T::from_client(request.client.clone())
            .await?
            .into_iter()
            .map(|v| {
                v.into_data(&request.datafields, request.counterdb.clone())
            })
            .collect();
        Ok(resources)
    }

    async fn request(
        &self,
        request: Request<'_>,
    ) -> (ProtoDataTableId, TableData) {
        let datatableid = request.datatable.0.clone();
        let data: DTEResult<Vec<ProtoRow>> = match request
            .datatable
            .1
            .command_name
            .as_str()
        {
            "version" => self.request_resource::<Version>(request).await,
            "cluster_status" => {
                self.request_resource::<ClusterStatus>(request).await
            }
            "vm_status" => self.request_resource::<VmStatus>(request).await,
            "vm_snapshot" => self.request_resource::<VmSnapshot>(request).await,
            "lxc_status" => self.request_resource::<LxcStatus>(request).await,
            "task" => self.request_resource::<Task>(request).await,
            "storage" => self.request_resource::<Storage>(request).await,

            // "replication" => self.request_resource::<Replication>(request).await,
            // "ceph_status" => self.request_resource::<CephStatus>(request).await,
            _ => {
                return (
                    datatableid,
                    Err(APIDTError::CommandNotFound(
                        request.datatable.0.to_string(),
                    )),
                )
            }
        }
        .tap_ok(|_| debug!("requesting {} successfull", &datatableid))
        .tap_err(|e| error!("requesting {} failed: {e}", &datatableid));

        (
            datatableid,
            data.map_err(APIDTError::Proxmox).map(|value| Annotated {
                value,
                warnings: Vec::new(),
            }),
        )
    }
}

#[async_trait::async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        info!("using Proxmox plugin");
        let client = self.config.get_client(self).await?;
        let counterdb = Arc::new(
            CounterDb::load(self.cache_dir.join("counters.json")).await?,
        );

        let requests = query
            .iter()
            .map(|(dtid, dfids)| {
                debug!("preparing request for {dtid}");
                let dt = ProtPlugin::get_datatable_id(dtid)
                    .try_get_from(&input.data_tables)?;
                let dfs = dfids
                    .iter()
                    .map(|dfid| {
                        ProtPlugin::get_datafield_id(dfid)
                            .try_get_from(&input.data_fields)
                            .map(|f| (dfid, f))
                            .map_err(Error::AgentUtils)
                    })
                    .collect::<Result<HashMap<_, _>>>()?;

                let request = Request {
                    client: client.clone(),
                    counterdb: counterdb.clone(),
                    datatable: (dtid, dt),
                    datafields: dfs,
                };

                Ok(self.request(request))
            })
            .collect::<Result<Vec<_>>>()?;

        info!("executing {} requests", requests.len());
        let data = stream::iter(requests)
            .buffer_unordered(query.len())
            .collect::<Vec<_>>()
            .await;
        info!("requests completed");
        let data = data.into_iter().collect::<DataMap>();

        if let Err(e) = counterdb.save().await {
            warn!("error saving counterdb: {e}");
        }

        Ok(data)
    }
}
