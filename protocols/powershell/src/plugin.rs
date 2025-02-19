/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{collections::HashMap, fmt::Write, path::PathBuf, sync::Arc};

use agent_utils::{KeyVault, TryGet};
use etc_base::{
    Annotated, AnnotatedResult, ProtoDataFieldId, ProtoDataTableId,
    ProtoQueryMap, ProtoRow,
};
use log::{debug, info, trace, warn};
use protocol::{CounterDb, DataFieldSpec, DataTableSpec, LocalPlugin};
use tap::TapFallible;

use crate::{
    error::{DTError, DTWarning, Result, TypeError, TypeResult},
    input::Input,
    Config, Error,
};

pub type Row = HashMap<String, String>;
pub type Table = Vec<Row>;
type TableData = AnnotatedResult<Vec<ProtoRow>, DTWarning, DTError>;
type DataMap = HashMap<ProtoDataTableId, TableData>;

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
}

#[async_trait::async_trait]
impl LocalPlugin for Plugin {
    type Error = Error;
    type TypeError = TypeError;
    type DTError = DTError;
    type DTWarning = DTWarning;

    type Input = Input;
    type Config = Config;

    const PROTOCOL: &'static str = "Powershell";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn show_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> Result<String> {
        let mut out = String::new();

        for (table, fields) in query.iter() {
            let table = input.data_tables.try_get(table)?;
            let fields = fields
                .iter()
                .map(|field| {
                    input
                        .data_fields
                        .try_get(field)
                        .map_err(Error::AgentUtils)
                        .map(|f| f.parameter_name.as_str())
                })
                .collect::<Result<Vec<_>>>()?;
            writeln!(
                out,
                "{} {} ({}) on {}: {}",
                Self::PROTOCOL,
                &table.command_name,
                &table.command_line,
                &table.shell_type,
                fields.join(", ")
            )?;
        }

        Ok(out)
    }

    fn get_tables(
        &self,
        input: &Input,
    ) -> TypeResult<HashMap<ProtoDataTableId, DataTableSpec>> {
        input
            .data_tables
            .iter()
            .map(|(dt_id, dt)| {
                let dfs = input
                    .data_table_fields
                    .try_get(dt_id)?
                    .iter()
                    .map(|df_id| {
                        input.data_fields.try_get(df_id).map(|df| (df_id, df))
                    })
                    .collect::<agent_utils::Result<HashMap<_, _>>>()?;
                Ok((
                    dt_id.clone(),
                    DataTableSpec {
                        name: dt_id.0.clone(),
                        keys: dfs
                            .iter()
                            .filter(|(_, v)| v.is_key)
                            .map(|(&k, _)| k.clone())
                            .collect(),
                        singleton: dt.singleton,
                        fields: dfs.into_keys().cloned().collect(),
                    },
                ))
            })
            .collect()
    }

    fn get_fields(
        &self,
        input: &Input,
    ) -> TypeResult<HashMap<ProtoDataFieldId, DataFieldSpec>> {
        input
            .data_fields
            .iter()
            .map(|(df_id, df)| {
                Ok((
                    df_id.clone(),
                    DataFieldSpec {
                        name: df.parameter_name.clone(),
                        input_type: df.get_type()?,
                    },
                ))
            })
            .collect::<TypeResult<HashMap<_, _>>>()
    }

    async fn run_queries(
        &self,
        input: &Input,
        config: &Config,
        query: &ProtoQueryMap,
    ) -> Result<DataMap> {
        info!("Using the winrm protocol");

        let mut session = config.new_session(&self.key_vault).await?;
        debug!("created session");
        debug!("created shell");
        info!("successfully logged in");

        let counter_file = self.cache_dir.join("winrm_counters.json");
        debug!("loading counters: {}", counter_file.display());
        let counter_db = CounterDb::load(counter_file.clone())
            .await
            .map_err(|e| Error::LoadCounters(counter_file.clone(), e))
            .map(Arc::new)?;
        debug!("loaded counters");

        let context = config.script_context();
        let mut data = HashMap::with_capacity(query.len());
        for (dt_id, df_ids) in query {
            let dt = input.data_tables.try_get(dt_id)?;
            let dfs = df_ids
                .iter()
                .map(|df_id| {
                    input
                        .data_fields
                        .try_get(df_id)
                        .map(|df| (df_id, df))
                        .map_err(Error::AgentUtils)
                })
                .collect::<Result<HashMap<_, _>>>()?;

            info!(
                "requesting {} with {} fields with {}",
                dt.command_name,
                dfs.len(),
                dt.shell_type
            );

            let script = match dt.shell_type.parse_command(
                &dt.command_line,
                dt.output_type,
                &context,
            ) {
                Ok(s) => s,
                Err(e) => {
                    data.insert(dt_id.clone(), Err(e));
                    continue;
                }
            };

            let output = session
                .run_ps(&script)
                .await
                .tap_ok(|out| {
                    trace!(
                        "output from command (exitcode = {}):\n{}",
                        out.exitcode,
                        &out.stdout
                    )
                })
                .tap_err(|e| warn!("error while executing command: {e}"));

            let table = output
                .map(|out| dt.output_type.parse_table(out))
                .and_then(std::convert::identity)
                .map(|t| dt.parse_table(t, dfs, counter_db.clone()))
                .map(|t| Annotated {
                    value: t,
                    warnings: Vec::new(),
                });

            data.insert(dt_id.clone(), table);
        }

        info!("all commands executed");
        if let Err(e) = counter_db.save().await {
            warn!("unable to save counters to {}: {e}", counter_file.display());
        }

        Ok(data)
    }
}
