/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
    net::IpAddr,
    sync::Arc,
};

use agent_utils::ip_lookup_one;
use futures::{stream, StreamExt};
use log::error;
use tap::{Pipe, TapFallible};
use value::Data;
use wmi_protocol::CounterDB;

use crate::{
    Config, ConnectionString, DTEResult, DTError, Error, FieldSpec,
    InstanceType, Result, Table, TableSpec,
};

use super::SqlPlugin;

#[derive(Debug, Clone)]
pub struct Plugin(Arc<CounterDB>);

impl Plugin {
    pub async fn from_protocol_plugin(
        prot_plugin: &crate::Plugin,
    ) -> Result<Self> {
        CounterDB::new(prot_plugin.cache_dir.join("sql_counters.json"))
            .await
            .map(Arc::new)
            .map(Self)
            .map_err(Error::CounterDbCreation)
    }

    fn transform_performance_counters(table: Table) -> DTEResult<Table> {
        // let static_values = ["__instance_name", "__object_name", "__object_instance"];
        // step 1: map object & instance to their counters
        let orginized_table =
            table.into_iter().try_fold::<_, _, DTEResult<_>>(
                HashMap::with_capacity(400),
                |mut accum, mut elem| {
                    accum
                        .entry((
                            elem.remove("__instance_name")
                                .ok_or(DTError::FieldNotFound(
                                    "__instance_name",
                                ))?
                                .trim_end()
                                .to_string(),
                            elem.remove("__object_name")
                                .ok_or(DTError::FieldNotFound("__object_name"))?
                                // depending on the counter, the objectname is prefexed with the instancename
                                // which is not buenõ
                                .split(':')
                                .last()
                                .unwrap()
                                .trim_end()
                                .to_string(),
                            elem.remove("__object_instance")
                                .ok_or(DTError::FieldNotFound(
                                    "__object_instance",
                                ))?
                                .trim_end()
                                .to_string(),
                        ))
                        .or_insert_with(|| Vec::with_capacity(600))
                        .push((
                            elem.remove("counter_name")
                                .ok_or(DTError::FieldNotFound("counter_name"))?
                                .trim_end()
                                .to_string(),
                            elem.remove("cntr_value")
                                .ok_or(DTError::FieldNotFound("cntr_value"))?,
                        ));
                    Ok(accum)
                },
            )?;
        // trace!("orginized_table: {orginized_table:#?}");

        // step 2. get default for all counters
        let counter_defaults = orginized_table
            .values()
            .flat_map(|counters| {
                counters
                    .iter()
                    .map(|(name, _value)| (name.clone(), String::new()))
            })
            .collect::<HashSet<(String, String)>>();
        // trace!("counter_defaults: {counter_defaults:#?}");

        // step 3: create a table based from step 1
        let transormed_table: Table = orginized_table
            .into_iter()
            .map(|((sql_instance, object_name, object_instance), counters)| {
                vec![
                    ("__instance_name".to_string(), sql_instance),
                    ("__object_name".to_string(), object_name),
                    ("__object_instance".to_string(), object_instance),
                ]
                .into_iter()
                // add all counters as default, to be overridden with actual counters
                // then there is not DataError::Missing for rows where this some counters do not apply
                .chain(counter_defaults.clone())
                .chain(counters)
                .collect()
            })
            .collect();
        // trace!("transformed_table: {transormed_table:#?}");

        Ok(transormed_table)
    }

    async fn discover_instances(
        &self,
        addr: IpAddr,
    ) -> Result<HashMap<InstanceType, u16>> {
        let mut instances = HashMap::new();
        let mut inst_iter = mssql_browser::browse_host(addr)
            .await
            .map_err(|e| Error::MsSqlBrowse(e.to_string()))?;

        while let Some(instance) = inst_iter
            .next()
            .map_err(|e| Error::MsSqlBrowse(e.to_string()))?
        {
            instances.insert(
                InstanceType::String(instance.instance_name),
                instance
                    .tcp_info
                    .ok_or_else(|| {
                        Error::MsSqlBrowse("No tcp info found".to_string())
                    })?
                    .port,
            );
        }

        Ok(instances)
    }

    async fn browse_instances(
        &self,
        addr: IpAddr,
        instances: Vec<InstanceType>,
    ) -> Result<HashMap<InstanceType, u16>> {
        let len = instances.len();
        instances
            .into_iter()
            .map(|inst| async {
                let port = match &inst {
                    InstanceType::Port(p) => *p,
                    InstanceType::Default => 1433,
                    InstanceType::String(s) => {
                        mssql_browser::browse_instance(addr, s)
                            .await
                            .map_err(|e| Error::MsSqlBrowse(e.to_string()))?
                            .tcp_info
                            .ok_or_else(|| {
                                Error::MsSqlBrowse(
                                    "No tcp info found".to_string(),
                                )
                            })?
                            .port
                    }
                };
                Ok((inst, port))
            })
            .pipe(stream::iter)
            .buffer_unordered(len)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect()
    }

    async fn get_instances(
        &self,
        config: Arc<Config>,
    ) -> Result<HashMap<InstanceType, u16>> {
        let addr = match config.ip {
            None => ip_lookup_one(&config.hostname).await?,
            Some(ip) => ip,
        };

        if config.instances.is_empty() {
            self.discover_instances(addr).await
        } else {
            self.browse_instances(addr, config.instances.clone()).await
        }
    }
}

#[async_trait::async_trait]
impl SqlPlugin for Plugin {
    fn name(&self) -> &'static str {
        "MSSQL"
    }

    async fn connection_string_per_instance(
        &self,
        base: ConnectionString,
        config: Arc<Config>,
    ) -> Result<HashMap<InstanceType, ConnectionString>> {
        tokio::time::timeout(
            std::time::Duration::from_secs(
                *(config.timeout.as_ref().unwrap_or(&10)) as u64,
            ),
            self.get_instances(config.clone()),
        )
        .await
        .map_err(|_| Error::MsSqlBrowse("Browsing timed out".to_string()))
        .tap_err(|_| {
            error!(
                r#"Browsing MSSQL instances timed out. \
               This can occur when the provided instances are invalid"#
            )
        })?
        .map(|insts| {
            insts
                .into_iter()
                .map(|(inst, p)| {
                    (
                        inst,
                        base.clone().with_arg(
                            "Server",
                            format!("{},{}", &config.hostname, p),
                        ),
                    )
                })
                .collect()
        })
    }
    fn construct_query(
        &self,
        datatable: &TableSpec,
        datafields: HashSet<&FieldSpec>,
    ) -> DTEResult<String> {
        if datatable.sql_table_name == "sys.dm_os_performance_counters" {
            let static_values =
                ["__instance_name", "__object_name", "__object_instance"];
            let clause = datafields
                .into_iter()
                .filter(|df| {
                    !static_values.contains(&df.column_request.as_str())
                })
                .map(|df| format!("counter_name = '{}'", &df.column_request))
                .collect::<Vec<_>>()
                .join("\n\t\tOR ");
            return Ok(format!(
                r#"SELECT @@SERVICENAME AS __instance_name,
                            object_name AS __object_name,
                            instance_name AS __object_instance,
                            counter_name, cntr_value
                    FROM sys.dm_os_performance_counters
                    WHERE {clause}"#
            ));
        }

        datatable
            .to_query(&datafields)
            .map_err(|e| DTError::ConstructQuery(Box::new(e)))
    }
    fn transform_table<'a>(
        &self,
        spec: &TableSpec,
        table: &'a Table,
    ) -> DTEResult<Cow<'a, Table>> {
        if spec.sql_table_name == "sys.dm_os_performance_counters" {
            return Self::transform_performance_counters(table.clone())
                .map(Cow::Owned);
        }

        Ok(Cow::Borrowed(table))
    }

    async fn save_counters(&self) -> Result<()> {
        self.0.save().await.map_err(Error::CounterDbSave)
    }
    /// This implementation panics if field.counter_type is not set. check beforehand
    fn parse_counter(
        &self,
        row: &mut HashMap<String, String>,
        field: &FieldSpec,
        base_key: &str,
    ) -> Data {
        return field.counter_type.as_ref().unwrap().get_wmi_counter(
            base_key,
            &field.column_name,
            self.0.clone(),
            row,
        );
    }
    fn parse_difference(
        &self,
        _row: &mut HashMap<String, String>,
        _field: &FieldSpec,
        _base_key: &str,
    ) -> Data {
        panic!("There is no such thing as a wmi counter with a difference")
    }
}

impl Display for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}
