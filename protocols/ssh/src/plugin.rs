/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

/* -*- tab-width: 4 -*- */

use agent_utils::{KeyVault, TryGet, TryGetFrom};
use async_ssh2_lite::{
    tokio::{self, io::AsyncReadExt},
    AsyncSession, TokioTcpStream,
};
use futures::{stream, StreamExt};
use log::info;
use std::{collections::HashMap, path::PathBuf, process::Stdio, sync::Arc};
use tap::Pipe;

use async_trait::async_trait;
use etc_base::{
    Annotated, AnnotatedResult, ProtoDataFieldId, ProtoDataTableId,
    ProtoQueryMap, ProtoRow, Warning,
};
use protocol::{DataFieldSpec, DataTableSpec, LocalPlugin};
use tokio::process::Command;

use crate::{Config, DTError, DTWarning, Error, Result};

type TableData = AnnotatedResult<Vec<ProtoRow>, DTWarning, DTError>;
pub type DataMap = HashMap<ProtoDataTableId, TableData>;

use sshparser_lib::{FieldSpec, Input, ParseRequest, TableSpec};
use std::fmt::Write;

use tokio::io::AsyncWriteExt;

pub struct Plugin {
    key_vault: KeyVault,
    cache_dir: PathBuf,
    parser_dir: PathBuf,
    log_level: u8,
}

impl Plugin {
    pub fn new(
        cache_dir: PathBuf,
        key_vault: KeyVault,
        parser_dir: PathBuf,
        log_level: u8,
    ) -> Self {
        Self {
            key_vault,
            cache_dir,
            parser_dir,
            log_level,
        }
    }

    pub fn counter_db(&self, tablespec: &TableSpec) -> PathBuf {
        const ILLEGAL_CHARS: [char; 1] = ['/'];
        self.cache_dir.join(format!(
            "ssh_{}_{}.json",
            tablespec.parser_name,
            tablespec.command_name.replace(ILLEGAL_CHARS, "_")
        ))
    }

    pub fn get_fieldspecs(
        &self,
        table_id: &ProtoDataTableId,
        input: &Input,
    ) -> Result<HashMap<ProtoDataFieldId, FieldSpec>> {
        let field_ids = input
            .data_table_fields
            .try_get(table_id)
            .map_err(Error::Specfile)?;

        field_ids
            .iter()
            .filter_map(|id| {
                Some((id.clone(), input.data_fields.get(id)?.clone()))
            })
            .collect::<HashMap<ProtoDataFieldId, FieldSpec>>()
            .pipe(Ok)
    }

    async fn get_data(
        &self,
        table_spec: &TableSpec,
        session: Arc<AsyncSession<TokioTcpStream>>,
        field_specs: HashMap<ProtoDataFieldId, FieldSpec>,
    ) -> TableData {
        let mut warnings = Vec::new();
        // Create SSH channel per request
        let mut ssh_channel = session
            .channel_session()
            .await
            .map_err(DTError::CreateChannel)?;
        if let Err(e) = ssh_channel.setenv("LANG", "C").await {
            let warn = Warning::warn(DTWarning::SetEnvVariable("LANG", e));
            warn.log();
            warnings.push(warn);
        }

        // Execute the commandline through ssh
        let command_line = &table_spec.command_line;

        log::info!("Executing command name: {}", &table_spec.command_name);
        log::trace!("Executing command: {}", command_line);
        ssh_channel
            .exec(command_line)
            .await
            .map_err(|e| DTError::Command(e, command_line.clone()))?;

        let mut command_output = String::new();
        ssh_channel
            .read_to_string(&mut command_output)
            .await
            .map_err(|e: std::io::Error| {
                DTError::ReadChannel(e, table_spec.command_line.clone())
            })?;

        let mut stderr = String::new();
        ssh_channel
            .stderr()
            .read_to_string(&mut stderr)
            .await
            .map_err(|e: std::io::Error| {
                DTError::ReadChannel(e, table_spec.command_line.clone())
            })?;
        log::trace!("stdout from command: {}", &command_output);
        log::trace!("stderr from command: {}", &stderr);
        if let Err(e) = ssh_channel.close().await {
            log::warn!("Could not close channel: {e}");
        }

        let exit_status = ssh_channel
            .exit_status()
            .map_err(DTError::RetrieveExitStatus)?;
        if exit_status != 0 {
            return Err(DTError::CommandFailed(exit_status, stderr));
        }
        drop(ssh_channel);

        // Prepare data to pass to the parser
        let par = ParseRequest {
            parser_name: table_spec.parser_name.clone(),
            command_output,
            field_specs,
            counter_db: self.counter_db(table_spec),
            log_level: self.log_level,
        };

        // Create the subprocess for the parser
        let mut child =
            Command::new(self.parser_dir.join(&table_spec.parser_bin))
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| {
                    DTError::SubProcess(table_spec.parser_bin.clone(), e)
                })?;

        // Pass commandline output to the parser
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(
                serde_json::to_string(&par)
                    .map_err(DTError::Json)?
                    .as_bytes(),
            )
            .await
            .map_err(|e| {
                DTError::SubProcess(table_spec.parser_bin.clone(), e)
            })?;
        drop(stdin);
        let output = child.wait_with_output().await.map_err(|e| {
            DTError::SubProcess(table_spec.parser_bin.clone(), e)
        })?;

        // Retrieve parser output
        let stdout = String::from_utf8(output.stdout).map_err(DTError::Utf8)?;
        let stderr = String::from_utf8(output.stderr).map_err(DTError::Utf8)?;
        log::trace!("parsed command output: {}", stdout);
        log::trace!("logging output: {}", stderr);

        let mut anno = serde_json::from_str::<
            AnnotatedResult<Vec<ProtoRow>, String, String>,
        >(&stdout)
        .map_err(DTError::Json)?
        .map_err(DTError::Parser)?
        .map_warning(DTWarning::Parser);

        anno.warnings.extend(warnings);
        // Transform parser output types and return
        Ok(anno)
    }
}

#[async_trait]
impl LocalPlugin for Plugin {
    type Error = Error;
    type TypeError = Error;
    type DTError = DTError;
    type DTWarning = DTWarning;

    type Input = Input;

    type Config = Config;

    const PROTOCOL: &'static str = "SSH";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn show_queries(
        &self,
        input: &Input,
        _query: &ProtoQueryMap,
    ) -> Result<String> {
        let out = String::new();
        for table_spec in input.data_tables.values() {
            writeln!(format!(
                "SSH: {}, {}",
                table_spec.command_name, table_spec.command_line
            ))
            .map_err(Error::ShowQueries)?;
        }
        Ok(out)
    }

    fn get_tables(
        &self,
        input: &Self::Input,
    ) -> Result<HashMap<ProtoDataTableId, DataTableSpec>> {
        input
            .data_tables
            .keys()
            .map(|dt_id| {
                let datafields = input
                    .data_table_fields
                    .get(dt_id)
                    .cloned()
                    .unwrap_or_default();
                (
                    dt_id.clone(),
                    DataTableSpec {
                        name: dt_id.0.clone(),
                        singleton: false,
                        keys: datafields
                            .iter()
                            .filter(|id| {
                                input
                                    .data_fields
                                    .get(id)
                                    .map_or(false, |field| field.is_key)
                            })
                            .cloned()
                            .collect(),
                        fields: datafields,
                    },
                )
            })
            .collect::<HashMap<ProtoDataTableId, DataTableSpec>>()
            .pipe(Ok)
    }

    fn get_fields(
        &self,
        input: &Self::Input,
    ) -> Result<HashMap<ProtoDataFieldId, DataFieldSpec>> {
        input
            .data_fields
            .iter()
            .map(|(df_id, metric_spec)| {
                (
                    df_id.clone(),
                    DataFieldSpec {
                        name: metric_spec.parameter_name.to_string(),
                        input_type: metric_spec.get_type(),
                    },
                )
            })
            .collect::<HashMap<ProtoDataFieldId, DataFieldSpec>>()
            .pipe(Ok)
    }

    async fn run_queries(
        &self,
        input: &Input,
        config: &Config,
        query: &ProtoQueryMap,
    ) -> Result<DataMap> {
        let session = config.get_session(&self.key_vault).await?;

        // Create empty vec to put our async requests in
        let mut requests = Vec::with_capacity(input.data_tables.len());

        // Check for sudo commands, create warning if sudo command was issued without correct config
        let mut sudo_warnings: DataMap = Default::default();
        for (table_id, field_ids) in query.iter() {
            log::trace!("creatibg request for {table_id:?}");
            let table_spec = table_id.try_get_from(&input.data_tables)?;

            let command_line = &table_spec.command_line;
            if command_line.contains("sudo ") && !config.options.allow_sudo {
                log::warn!("Skipping command {} that requires sudo permissions, since allow sudo has not been enabled", &table_spec.command_name);
                sudo_warnings.insert(
                    table_id.clone(),
                    Ok(Annotated {
                        value: Vec::with_capacity(0),
                        warnings: vec![etc_base::Warning {
                            verbosity: logger::Verbosity::Warning,
                            message: DTWarning::SudoNotAllowed(),
                        }],
                    }),
                );
                continue;
            }

            // Get fieldspecs that match our table_id
            let field_specs = field_ids
                .iter()
                .map(|fid| {
                    Ok((
                        fid.clone(),
                        fid.try_get_from(&input.data_fields)?.clone(),
                    ))
                })
                .collect::<Result<HashMap<ProtoDataFieldId, FieldSpec>>>()?;

            // Put a pair of table_id and get_data call for our table into the requests vector as an async block
            requests.push(async {
                (
                    table_id.clone(),
                    self.get_data(table_spec, session.clone(), field_specs)
                        .await,
                )
            })
        }
        log::trace!(
            "All data requests succesfully prepared, start executing..."
        );
        // Done looping through all table_id's, awaiting all get_data calls.
        info!(
            "requesting data with max {} channels",
            config.connectivity.max_sessions
        );
        let mut data = stream::iter(requests)
            .buffer_unordered(config.connectivity.max_sessions as usize)
            .collect::<HashMap<ProtoDataTableId, TableData>>()
            .await;
        log::trace!("Data requests executed");
        // add the sudo-related warnings to the data we created
        data.extend(sudo_warnings);

        //Return the data
        Ok(data)
    }
}
