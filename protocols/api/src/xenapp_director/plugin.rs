/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::sync::Arc;

use agent_utils::{KeyVault, TryGetFrom};
use etc_base::{Annotated, ProtoDataFieldId, ProtoQueryMap};
use futures::{stream, StreamExt};
use log::info;
use value::DataError;

use crate::error::Result as APIResult;
use crate::input::{FieldSpec, TableSpec, ValueTypes};
use crate::plugin::TableData;
use crate::{plugin::DataMap, APIPlugin, Input, Plugin as ProtPlugin};

use super::{response, Client, Config, DTError, Error, Result};

pub struct Plugin {
    key_vault: KeyVault,
    config: Config,
}

impl Plugin {
    pub fn new(key_vault: KeyVault, config: Config) -> Self {
        Self { key_vault, config }
    }

    async fn request_monitor_machine(
        &self,
        client: Arc<Client>,
        dt: &TableSpec,
        dfs: HashMap<&ProtoDataFieldId, &FieldSpec>,
    ) -> TableData {
        const ENDPOINT: &str = "Monitor/OData/v2/Data/Machines";
        let response: response::CitrixMonitorData =
            client.request(ENDPOINT, &dt.command_line).await?;

        let load_properties = response
            .entry
            .link
            .into_iter()
            .find_map(|l| l.inline)
            .ok_or(DTError::PropertyNotExpanded)?
            .entry
            .content
            .properties;
        let machine_properties = response.entry.content.properties;
        let row = dfs
            .into_iter()
            .map(|(dfid, df)| {
                (
                    dfid.clone(),
                    match df.parameter_header.as_str() {
                        "id" => load_properties.effective_load_index.to_value(),
                        "effective_load_index" => {
                            load_properties.effective_load_index.to_value()
                        }
                        "cpu" => load_properties.cpu.to_value(),
                        "memory" => load_properties.memory.to_value(),
                        "disk" => load_properties.disk.to_value(),
                        "network" => load_properties.network.to_value(),
                        "session_count" => {
                            load_properties.session_count.to_value()
                        }
                        "machine_id" => load_properties.machine_id.to_value(),
                        "created_date" => {
                            load_properties.created_date.to_value()
                        }

                        "machine" => machine_properties.id.to_value(),
                        "sid" => machine_properties.sid.to_value(),
                        "name" => machine_properties.name.to_value(),
                        "dns_name" => machine_properties.dns_name.to_value(),
                        "lifecycle_state" => {
                            let value = &machine_properties.lifecycle_state;
                            match &df.values {
                                None => value.to_value(),
                                Some(ValueTypes::String(_)) => {
                                    Err(DataError::External(
                                        "string values found for int enum"
                                            .to_string(),
                                    ))
                                }
                                Some(ValueTypes::Integer(choices)) => {
                                    value.to_intenum(choices.clone())
                                }
                            }
                        }

                        "ip_address" => {
                            machine_properties.ip_address.to_value()
                        }
                        "hosted_machine_id" => {
                            machine_properties.hosted_machine_id.to_value()
                        }
                        "hosting_server_name" => {
                            machine_properties.hosting_server_name.to_value()
                        }
                        "hosted_machine_name" => {
                            machine_properties.hosted_machine_name.to_value()
                        }
                        "is_assigned" => {
                            machine_properties.is_assigned.to_value()
                        }
                        "is_in_maintenance_mode" => {
                            machine_properties.is_in_maintenance_mode.to_value()
                        }
                        "is_pending_update" => {
                            machine_properties.is_pending_update.to_value()
                        }
                        "agent_version" => {
                            machine_properties.agent_version.to_value()
                        }
                        "associated_user_full_names" => machine_properties
                            .associated_user_full_names
                            .to_value(),
                        "associated_user_names" => {
                            machine_properties.associated_user_names.to_value()
                        }
                        "associated_user_upns" => {
                            machine_properties.associated_user_upns.to_value()
                        }
                        "current_registration_state" => {
                            let value =
                                &machine_properties.current_registration_state;
                            match &df.values {
                                None => value.to_value(),
                                Some(ValueTypes::String(_)) => {
                                    Err(DataError::External(
                                        "string values found for int enum"
                                            .to_string(),
                                    ))
                                }
                                Some(ValueTypes::Integer(choices)) => {
                                    value.to_intenum(choices.clone())
                                }
                            }
                        }

                        "last_deregistered_code" => {
                            machine_properties.last_deregistered_code.to_value()
                        }
                        "current_power_state" => {
                            machine_properties.current_power_state.to_value()
                        }
                        "current_session_count" => {
                            machine_properties.current_session_count.to_value()
                        }
                        "controller_dns_name" => {
                            machine_properties.controller_dns_name.to_value()
                        }
                        "functional_level" => {
                            machine_properties.functional_level.to_value()
                        }
                        "windows_connection_setting" => machine_properties
                            .windows_connection_setting
                            .to_value(),
                        "is_preparing" => {
                            machine_properties.is_preparing.to_value()
                        }
                        "fault_state" => {
                            machine_properties.fault_state.to_value()
                        }
                        "os_type" => machine_properties.os_type.to_value(),
                        "current_load_index_id" => {
                            machine_properties.current_load_index_id.to_value()
                        }
                        "catalog_id" => {
                            machine_properties.catalog_id.to_value()
                        }
                        "desktop_group_id" => {
                            machine_properties.desktop_group_id.to_value()
                        }
                        "hypervisor_id" => {
                            machine_properties.hypervisor_id.to_value()
                        }
                        "hash" => machine_properties.hash.to_value(),
                        "machine_role" => {
                            let value = &machine_properties.machine_role;
                            match &df.values {
                                None => value.to_value(),
                                Some(ValueTypes::String(_)) => {
                                    Err(DataError::External(
                                        "string values found for int enum"
                                            .to_string(),
                                    ))
                                }
                                Some(ValueTypes::Integer(choices)) => {
                                    value.to_intenum(choices.clone())
                                }
                            }
                        }

                        "registration_state_change_date" => machine_properties
                            .registration_state_change_date
                            .to_value(),
                        "last_deregistered_date" => {
                            machine_properties.last_deregistered_date.to_value()
                        }
                        "powered_on_date" => {
                            machine_properties.powered_on_date.to_value()
                        }
                        "power_state_change_date" => machine_properties
                            .power_state_change_date
                            .to_value(),
                        "failure_date" => {
                            machine_properties.failure_date.to_value()
                        }
                        "machine_created_date" => {
                            machine_properties.created_date.to_value()
                        }
                        "modified_date" => {
                            machine_properties.modified_date.to_value()
                        }

                        _ => Err(DataError::Missing),
                    },
                )
            })
            .collect();

        Ok(Annotated {
            value: vec![row],
            warnings: Vec::new(),
        })
    }

    async fn request_datatable(
        &self,
        client: Arc<Client>,
        dt: &TableSpec,
        dfs: HashMap<&ProtoDataFieldId, &FieldSpec>,
    ) -> TableData {
        match dt.command_name.as_str() {
            "Monitor.Model.V2.Machine" => {
                self.request_monitor_machine(client, dt, dfs).await
            }
            _ => Err(crate::error::DTError::CommandNotFound(
                dt.command_line.clone(),
            )),
        }
    }
}

#[async_trait::async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        info!("using xenapp plugin");
        let client =
            Arc::new(Client::new(&self.config, self.key_vault.clone()).await?);
        info!("client succesfully initialized");

        let futures = query
            .iter()
            .map(|(dtid, dfids)| {
                let dt = ProtPlugin::get_datatable_id(dtid)
                    .try_get_from(&input.data_tables)?;
                let dfs = dfids
                    .iter()
                    .map(|df_id| {
                        Ok((
                            df_id,
                            ProtPlugin::get_datafield_id(df_id)
                                .try_get_from(&input.data_fields)
                                .map_err(Error::AgentUtils)?,
                        ))
                    })
                    .collect::<Result<HashMap<_, _>>>()?;
                Ok(async {
                    (
                        dtid.clone(),
                        self.request_datatable(client.clone(), dt, dfs).await,
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?;
        info!("scheduled {} requests", futures.len());

        let data = stream::iter(futures)
            .buffer_unordered(query.len())
            .collect()
            .await;

        Ok(data)
    }
}
