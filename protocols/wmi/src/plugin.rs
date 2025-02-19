/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::HashMap;
use std::fmt::Write;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use log::{debug, info, warn};
use tokio::time;

use agent_utils::{KeyVault, TryGet, TryGetFrom};
use etc_base::{
    Annotated, AnnotatedResult, DataFieldId, DataTableId, ProtoDataFieldId,
    ProtoDataTableId, ProtoQueryMap, ProtoRow, Protocol,
};
use protocol::{DataFieldSpec, DataTableSpec, LocalPlugin};

use crate::counters::{CounterDB, COUNTER_VARIABLES, REQUIRES_BASE};
use crate::error::{TypeError, TypeResult, WMIDTError};
use crate::input::FieldSpec;
use crate::{Config, Input, Result, WMIError};

type TableData = AnnotatedResult<Vec<ProtoRow>, WMIDTError, WMIDTError>;
pub type DataMap = HashMap<ProtoDataTableId, TableData>;

pub struct Plugin {
    key_vault: KeyVault,
    cache_dir: PathBuf,
}

impl Plugin {
    pub fn new(cache_dir: PathBuf, key_vault: KeyVault) -> Self {
        Self {
            key_vault,
            cache_dir,
        }
    }

    fn get_datatable_id(dt_id: &ProtoDataTableId) -> DataTableId {
        DataTableId(Protocol(Self::PROTOCOL.to_string()), dt_id.clone())
    }
    fn get_datafield_id(df_id: &ProtoDataFieldId) -> DataFieldId {
        DataFieldId(Protocol(Self::PROTOCOL.to_string()), df_id.clone())
    }
}

#[async_trait]
impl protocol::LocalPlugin for Plugin {
    type Error = WMIError;
    type TypeError = TypeError;
    type DTError = WMIDTError;
    type DTWarning = WMIDTError;

    type Input = Input;
    type Config = Config;

    const PROTOCOL: &'static str = "WMI";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn show_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> Result<String> {
        let mut out = String::new();
        for table_id in query.keys() {
            let command = input
                .data_tables
                .try_get(&Self::get_datatable_id(table_id))?;
            writeln!(
                out,
                "WMI (Plugin: {:?}): {}/{}",
                &command.instance_plugin,
                &command.namespace,
                &command.classname
            )?;
        }
        Ok(out)
    }

    fn get_tables(
        &self,
        input: &Self::Input,
    ) -> TypeResult<HashMap<ProtoDataTableId, DataTableSpec>> {
        let res = input
            .data_tables
            .keys()
            .map(|dt_id| {
                let datafields = input
                    .data_table_fields
                    .get(dt_id)
                    .cloned()
                    .unwrap_or_default();
                (
                    dt_id.1.clone(),
                    DataTableSpec {
                        name: dt_id.1 .0.clone(),
                        singleton: false,
                        keys: datafields
                            .iter()
                            .map(|id| (id, input.data_fields.get(id)))
                            .filter_map(|(id, field)| {
                                if let Some(field) = field {
                                    match field.is_key {
                                        true => Some(id),
                                        false => None,
                                    }
                                } else {
                                    None
                                }
                            })
                            .map(|id| id.1.clone())
                            .collect(),
                        fields: datafields.into_iter().map(|id| id.1).collect(),
                    },
                )
            })
            .collect();
        Ok(res)
    }

    fn get_fields(
        &self,
        input: &Self::Input,
    ) -> TypeResult<HashMap<ProtoDataFieldId, DataFieldSpec>> {
        input
            .data_fields
            .iter()
            .map(|(df_id, field_spec)| {
                Ok((
                    df_id.1.clone(),
                    DataFieldSpec {
                        name: field_spec.property_name.clone(),
                        input_type: field_spec.get_type()?,
                    },
                ))
            })
            .collect::<TypeResult<HashMap<ProtoDataFieldId, DataFieldSpec>>>()
    }

    async fn run_queries(
        &self,
        input: &Input,
        config: &Config,
        query: &ProtoQueryMap,
    ) -> Result<DataMap> {
        info!("Using the wmi protocol");
        // info!("config from wato: {:?}", &config);

        let counter_file = self.cache_dir.join("wmi_counter_timestamps.json");
        let counterdb = Arc::new(CounterDB::new(counter_file).await?);

        let mut session = config.get_session(&self.key_vault).await?;
        info!("winrm session created");
        // fake login
        // let shell = session.shell().await?;
        // info!("login successfull");

        let mut data: DataMap = HashMap::new();
        for (dt_id, df_ids) in query {
            info!(
                "creating request for {} with {} fields",
                &dt_id,
                df_ids.len()
            );

            let df_ids = df_ids
                .iter()
                .map(|df_id| {
                    Ok((
                        df_id.clone(),
                        Self::get_datafield_id(df_id)
                            .try_get_from(&input.data_fields)?,
                    ))
                })
                .collect::<Result<HashMap<ProtoDataFieldId, &FieldSpec>>>()?;
            let fieldnames = get_fieldnames(&df_ids);
            let class = (
                dt_id,
                Self::get_datatable_id(dt_id)
                    .try_get_from(&input.data_tables)?,
            );

            let method = config.get_method();
            let mut wmi_res = method
                .exec_query(
                    &mut session,
                    &class.1.classname,
                    &fieldnames,
                    &class.1.namespace,
                )
                .await;
            let mut retries = config.retries.unwrap_or(0);

            while wmi_res.is_err() {
                if retries == 0 {
                    break;
                }

                session = config.get_session(&self.key_vault).await?;
                time::sleep(Duration::from_secs(1)).await;
                wmi_res = method
                    .exec_query(
                        &mut session,
                        &class.1.classname,
                        &fieldnames,
                        &class.1.namespace,
                    )
                    .await;
                retries -= 1;
            }
            let wmi_res = wmi_res;
            debug!("result of {}: {:?}", &class.1.classname, &wmi_res);

            data.insert(
                class.0.clone(),
                match wmi_res {
                    Err(e) => {
                        warn!(
                            "An error occured while requesting {}: {:?}",
                            &class.0, &e
                        );
                        session = config.get_session(&self.key_vault).await?;
                        Err(e)
                    }
                    Ok(wmi_res) => {
                        let mut idx: u32 = 0;
                        let mut data = Vec::new();

                        for wmi_obj in wmi_res {
                            let mut row = HashMap::new();
                            let base_key = format!(
                                "{}_{}",
                                &class.1.classname,
                                df_ids
                                    .iter()
                                    .filter(|(_, f)| f.is_key)
                                    .map(|(_, f)| {
                                        if let Some(k) = wmi_obj
                                            .get(&f.property_name.clone())
                                        {
                                            k.to_string()
                                        } else {
                                            idx += 1;
                                            idx.to_string()
                                        }
                                    })
                                    .collect::<Vec<String>>()
                                    .join("_")
                            );

                            for (fieldid, field) in df_ids.iter() {
                                row.insert(
                                    fieldid.clone(),
                                    field
                                        .parse_var(
                                            &wmi_obj,
                                            counterdb.clone(),
                                            &base_key,
                                            &config.quircks,
                                        )
                                        .await,
                                );
                            }
                            data.push(row);
                        }

                        Ok(Annotated {
                            value: data,
                            warnings: Vec::new(),
                        })
                    }
                },
            );
        }
        info!("All requests executed");

        counterdb.save().await?;

        Ok(data)
    }
}

fn get_fieldnames(
    fields: &HashMap<ProtoDataFieldId, &FieldSpec>,
) -> Vec<String> {
    let counters: Vec<_> = fields
        .values()
        .filter(|f| f.counter_type.is_some())
        .collect();
    fields
        .values()
        .map(|f| f.property_name.clone())
        .chain(
            if !counters.is_empty() {
                COUNTER_VARIABLES.iter()
            } else {
                [].iter()
            }
            .cloned(),
        )
        .chain(
            counters
                .iter()
                .filter(|f| {
                    REQUIRES_BASE.contains(f.counter_type.as_ref().unwrap())
                })
                .map(|f| format!("{}_Base", f.property_name)),
        )
        .collect()
}
