/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::{
    borrow::Cow,
    collections::{hash_map::Entry, HashMap, HashSet},
    fmt::Write,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use force_send_sync::Send;
use futures::FutureExt;
use itertools::Itertools;
use log::{debug, error, info, trace, warn};
use odbc_api::{
    buffers::TextRowSet, Connection, ConnectionOptions, Cursor, Environment,
    ResultSetMetadata,
};

use agent_utils::{KeyVault, TryGet};
use etc_base::{
    Annotated, AnnotatedResult, ProtoDataFieldId, ProtoDataTableId,
    ProtoQueryMap, ProtoRow, Warning,
};
use logger::Verbosity;
use protocol::{DataFieldSpec, DataTableSpec, LocalPlugin};
use tap::{Pipe, Tap, TapFallible};
use value::Data;
use wmi_protocol::CounterDB;

use crate::{
    config::{Config, InstanceType},
    error::{DTEResult, DTError, DTWResult, DTWarning, Error, Result},
    input::{FieldSpec, Input, TableSpec},
    sqlplugin::SqlPlugin,
};

pub type Table = Vec<HashMap<String, String>>;
type DataTable = DTWResult<Vec<HashMap<ProtoDataFieldId, Data>>>;
type TableData = AnnotatedResult<Vec<ProtoRow>, DTWarning, DTError>;
type DataMap = HashMap<ProtoDataTableId, TableData>;
type SConnection<'a> = force_send_sync::Send<odbc_api::Connection<'a>>;

lazy_static::lazy_static! {
    pub static ref ENV: Environment = Environment::new().unwrap();
}
const BATCH_SIZE: usize = 2048;

pub struct Plugin {
    pub key_vault: KeyVault,
    pub cache_dir: PathBuf,
}

impl Plugin {
    pub fn new(cache_dir: PathBuf, key_vault: KeyVault) -> Self {
        Self {
            key_vault,
            cache_dir,
        }
    }
}

#[derive(Debug, Clone)]
struct SqlRequest {
    pub general_queries:
        Arc<HashMap<ProtoDataTableId, (TableSpec, HashSet<ProtoDataFieldId>)>>,
    pub database_queries:
        Arc<HashMap<ProtoDataTableId, (TableSpec, HashSet<ProtoDataFieldId>)>>,
    pub datafields: Arc<HashMap<ProtoDataFieldId, FieldSpec>>,
    pub database_query: Arc<Option<String>>,
    pub sql_plugin: Arc<dyn SqlPlugin>,
    pub instance: InstanceType,
    pub connection_string: String,
}

impl SqlRequest {
    fn connect(&self, database: Option<&str>) -> Result<SConnection<'_>> {
        let connection_string = match database {
            None => self.connection_string.to_string(),
            Some(database) => {
                format!("{};Database={}", self.connection_string, database)
            }
        };
        ENV.connect_with_connection_string(
            &connection_string,
            ConnectionOptions::default(),
        )
        // while we have to mark it as unsafe. it should be safe, if the driver follows the odbc standard
        // https://docs.rs/odbc-api/0.57.0/odbc_api/struct.Connection.html#method.promote_to_send
        .map(|conn| unsafe { conn.promote_to_send() })
        .map_err(|e| Error::Connection(self.instance.clone(), e))
    }

    fn query(
        &self,
        connection: &SConnection<'_>,
        query: &str,
    ) -> DTEResult<Table> {
        debug!("[{}] executing query: {query}", self.instance);
        let mut cursor = connection
            .execute(query, ())
            .map_err(DTError::FailedQuery)?
            .ok_or(DTError::EmptyResult)?;

        let headers: Vec<String> = cursor
            .column_names()
            .map_err(DTError::Metadata)?
            .collect::<std::result::Result<_, _>>()
            .map_err(DTError::Metadata)?;
        trace!("[{}] received headers: {headers:?}", self.instance);

        let mut buffer = TextRowSet::for_cursor(BATCH_SIZE, &mut cursor, None)
            .map_err(DTError::Metadata)?;
        let mut cursor = cursor
            .bind_buffer(&mut buffer)
            .map_err(DTError::BufferBind)?;

        let mut table = Vec::new();
        while let Some(batch) = cursor.fetch().map_err(DTError::FetchRow)? {
            table.reserve(batch.num_rows());
            for row_idx in 0..batch.num_rows() {
                table.push(
                    (0..batch.num_cols())
                        .map(|col_idx| {
                            (
                                headers.get(col_idx).unwrap().to_string(),
                                String::from_utf8_lossy(
                                    batch.at(col_idx, row_idx).unwrap_or(&[]),
                                )
                                .to_string(),
                            )
                        })
                        .collect(),
                )
            }
        }

        trace!("[{}] recieved {} rows", self.instance, table.len());
        Ok(table)
    }

    fn get_databases(
        &self,
        connection: &SConnection<'_>,
    ) -> DTEResult<Option<Vec<String>>> {
        if self.database_query.is_none() {
            return Ok(None);
        }

        let query = (*self.database_query).as_ref().unwrap();
        let results = self.query(connection, query.as_str())?;

        results
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .next()
                    .map(|(_, v)| v)
                    .ok_or(DTError::NoDatabaseColumn)
            })
            .collect::<DTEResult<Vec<String>>>()
            .map(Some)
    }

    fn query_datatable(
        &self,
        connection: &Send<Connection>,
        tablespec: &TableSpec,
        datafields: &HashSet<ProtoDataFieldId>,
    ) -> DataTable {
        info!(
            "[{}]: Querying {}",
            self.instance,
            tablespec
                .sql_table_query
                .as_deref()
                .unwrap_or(tablespec.sql_table_name.as_str())
        );
        let fieldspecs: HashMap<&ProtoDataFieldId, &FieldSpec> = datafields
            .iter()
            .map(|df_id| (df_id, self.datafields.get(df_id).unwrap()))
            .collect();
        let query: String = self.sql_plugin.construct_query(
            tablespec,
            fieldspecs.values().cloned().collect(),
        )?;

        let datatable = self
            .query(connection, &query)
            .tap_err(|e| warn!("query failed: {e}"))
            .map_err(DTWarning::DTError)?;
        trace!("[{}] received table: {datatable:#?}", self.instance);
        let transformed = self
            .sql_plugin
            .transform_table(tablespec, &datatable)
            .tap_err(|e| {
                warn!(
                    "[{}] An error occured transforming table: {e}",
                    self.instance
                )
            })?;
        let datatable = match transformed {
            Cow::Borrowed(_) => datatable,
            Cow::Owned(dt) => dt.tap(|datatable| {
                trace!("[{}] transformed table: {datatable:#?}", self.instance)
            }),
        };

        let datatable = datatable
            .into_iter()
            .map(|mut row| {
                let base_key = fieldspecs
                    .values()
                    .filter(|&df| df.is_key)
                    .map(|df| {
                        row.get(&df.column_name).cloned().unwrap_or_default()
                    })
                    .sorted()
                    .collect::<Vec<_>>()
                    .join("_");

                fieldspecs
                    .iter()
                    .map(|(df_id, df)| {
                        (
                            (*df_id).clone(),
                            self.sql_plugin
                                .parse_value(&mut row, df, &base_key)
                                // this woule mean that we wanted to calculate a counter without a value, which is very possible given our pivot
                                .tap_err(|e| {
                                    if df.counter_type.is_none()
                                        && !e.to_string().contains("())")
                                    {
                                        warn!(
                                        "[{}] Could not parse {} to a {}: {e}",
                                        &self.instance,
                                        &df.column_name,
                                        df.get_type()
                                            .map(|t| t.to_string())
                                            .unwrap_or_else(|e| format!(
                                                "Could not get type: {e}"
                                            ))
                                    )
                                    }
                                }),
                        )
                    })
                    .collect()
            })
            .collect();

        Ok(datatable)
    }

    fn query_dbspecific(
        self,
        database: String,
    ) -> Result<HashMap<ProtoDataTableId, DataTable>> {
        debug!("[{}]: Switching to database: {database}", &self.instance);
        let connection = self.connect(Some(&database)).tap_err(|e| {
            error!(
                "Cannot connect to database {database} in instance {}: {e}",
                &self.instance
            )
        })?;

        Ok(self
            .database_queries
            .iter()
            .map(|(df_id, (dt, dfs))| {
                (df_id.clone(), self.query_datatable(&connection, dt, dfs))
            })
            .collect())
    }

    fn query_instance(self) -> Result<HashMap<ProtoDataTableId, DataTable>> {
        info!("[{}] starting connection", &self.instance);
        trace!(
            "[{}] with connection_string: {}",
            &self.instance,
            &self.connection_string
        );
        let connection = match self.connect(None) {
            Ok(conn) => conn,
            Err(e) => {
                error!("[{}] Cannot connect to instance: {e}", &self.instance);
                return Err(e);
            }
        };

        let databases = self
            .get_databases(&connection)
            .map_err(|e| {
                warn!(
                    "[{}]: Unable to retrieve databases on instance: {e}",
                    &self.instance
                );
                Error::DatabaseQuery(self.instance.clone(), Box::new(e))
            })?
            .unwrap_or_default();

        if !databases.is_empty() {
            info!(
                "[{}]: Found {} databases on instance: {:?}",
                &self.instance,
                databases.len(),
                &databases
            );
        }

        let db_handles = databases
            .into_iter()
            .map(|db| {
                let cloned = self.clone();
                tokio::task::spawn_blocking(move || cloned.query_dbspecific(db))
            })
            .collect::<Vec<_>>();

        let mut data: HashMap<_, _> = self
            .general_queries
            .iter()
            .map(|(df_id, (dt, dfs))| {
                (df_id.clone(), self.query_datatable(&connection, dt, dfs))
            })
            .collect();
        debug!("[{}]: Generic Queries done", &self.instance);
        data.reserve(self.database_queries.len());

        for handle in db_handles {
            while !handle.is_finished() {
                std::thread::sleep(Duration::from_millis(100));
            }
            let db_data = handle.now_or_never().unwrap().unwrap();

            let db_data = match db_data {
                Err(_) => continue,
                Ok(d) => d,
            };

            for (dt_id, dt_res) in db_data {
                match data.entry(dt_id) {
                    Entry::Vacant(entry) => {
                        entry.insert(dt_res);
                    }
                    Entry::Occupied(entry) => {
                        let (dt_id, mut datatable) = entry.remove_entry();
                        datatable = match (datatable, dt_res) {
                            (Err(e), _) => Err(e),
                            (_, Err(e)) => Err(e),
                            (Ok(mut table1), Ok(table2)) => {
                                table1.extend(table2);
                                Ok(table1)
                            }
                        };
                        data.insert(dt_id, datatable);
                    }
                };
            }
        }
        debug!("[{}]: Database Queries done", &self.instance);

        info!("[{}]: All queries completed", &self.instance);
        Ok(data)
    }
}

#[async_trait::async_trait]
impl LocalPlugin for Plugin {
    type Error = Error;
    type TypeError = Error;
    type DTError = DTError;
    type DTWarning = DTWarning;

    type Input = Input;
    type Config = Config;

    const PROTOCOL: &'static str = "SQL";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn show_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> Result<String> {
        let mut out = String::new();
        for (dt_id, df_ids) in query {
            let table = input.data_tables.try_get(dt_id)?;
            let datafields: HashSet<&FieldSpec> = df_ids
                .iter()
                .map(|df_id| {
                    input.data_fields.try_get(df_id).map_err(Error::AgentUtils)
                })
                .collect::<Result<HashSet<&FieldSpec>>>()?;
            writeln!(
                out,
                "SQL (Plugin: {}): {}",
                table.plugin.unwrap_or_default(),
                table.to_query(&datafields)?
            )?;
        }
        Ok(out)
    }

    fn get_tables(
        &self,
        input: &Self::Input,
    ) -> Result<HashMap<ProtoDataTableId, DataTableSpec>> {
        input
            .data_tables
            .iter()
            .map(|(dt_id, dt)| {
                dt.fields
                    .iter()
                    .map(|df_id| {
                        input
                            .data_fields
                            .try_get(df_id)
                            .map(|df| (df_id, df))
                            .map_err(Error::AgentUtils)
                    })
                    .collect::<Result<HashMap<_, _>>>()
                    .map(|dfs| DataTableSpec {
                        name: dt
                            .sql_table_query
                            .clone()
                            .unwrap_or_else(|| dt.sql_table_name.clone()),
                        singleton: !dt.is_table,
                        keys: dfs
                            .iter()
                            .filter(|(_df_id, df)| df.is_key)
                            .map(|(df_id, _df)| (*df_id).clone())
                            .collect(),
                        fields: dfs
                            .keys()
                            .map(|df_id| (*df_id).clone())
                            .collect(),
                    })
                    .map(|dts| (dt_id.clone(), dts))
            })
            .collect()
    }

    fn get_fields(
        &self,
        input: &Self::Input,
    ) -> Result<HashMap<ProtoDataFieldId, DataFieldSpec>> {
        input
            .data_fields
            .iter()
            .map(|(df_id, df)| {
                Ok((
                    df_id.clone(),
                    DataFieldSpec {
                        name: df.column_name.clone(),
                        input_type: df.get_type()?,
                    },
                ))
            })
            .collect()
    }

    async fn run_queries(
        &self,
        input: &Input,
        config: &Config,
        query: &ProtoQueryMap,
    ) -> Result<DataMap> {
        let config = Arc::new(config.clone());
        debug!("config: {config:#?}");
        debug!(
            "loading wmi counters: {}",
            self.cache_dir.join("sql_counters.json").display()
        );

        let datatables: HashMap<
            ProtoDataTableId,
            (TableSpec, HashSet<ProtoDataFieldId>),
        > = query
            .iter()
            .map(|(dt_id, dfs)| {
                input
                    .data_tables
                    .try_get(dt_id)
                    .map(|dt| (dt_id.clone(), (dt.clone(), dfs.clone())))
                    .map_err(Error::AgentUtils)
            })
            .collect::<Result<_>>()?;
        let datafields: Arc<HashMap<ProtoDataFieldId, FieldSpec>> = query
            .values()
            .flatten()
            .map(|df_id| {
                input
                    .data_fields
                    .try_get(df_id)
                    .map(|df| (df_id.clone(), df.clone()))
                    .map_err(Error::AgentUtils)
            })
            .collect::<Result<HashMap<ProtoDataFieldId, FieldSpec>>>()?
            .pipe(Arc::new);

        trace!(
            "datatables to request: {:#?}",
            datatables.values().collect::<Vec<_>>()
        );

        let sql_plugin = datatables
            .values()
            .next()
            .and_then(|dt| dt.0.plugin)
            .unwrap_or_default()
            .get_plugin(self)
            .await?;

        // general queries: do not care about the connected database. will be executed right away
        // database_queries: want to be executed in every database. a new connection will be created for every database and executed there
        let (general_queries, database_queries): (
            Arc<HashMap<_, _>>,
            Arc<HashMap<_, _>>,
        ) = datatables
            .into_iter()
            .partition(|(_dt_id, dt)| dt.0.database_query.is_none())
            .pipe(|(gq, dq)| (Arc::new(gq), Arc::new(dq)));

        // will be used to retrieve the databases from the server on initial conenction
        let database_query = Arc::new(
            database_queries
                .values()
                .next()
                .map(|dt| dt.0.database_query.as_ref().unwrap().clone()),
        );
        let connection_strings = config
            .clone()
            .generic_connectionstring(sql_plugin.clone(), &self.key_vault)
            .await?;

        info!(
            "{} queries ({} generic, {} database) to be executed on {} instances using the {} plugin",
            general_queries.len() + database_queries.len(),
            general_queries.len(), database_queries.len(),
            connection_strings.len(), sql_plugin
        );

        let handles = connection_strings
            .into_iter()
            .map(|(instance, connection_string)| {
                let request = SqlRequest {
                    general_queries: general_queries.clone(),
                    database_queries: database_queries.clone(),
                    datafields: datafields.clone(),
                    database_query: database_query.clone(),
                    sql_plugin: sql_plugin.clone(),
                    instance,
                    connection_string,
                };
                tokio::task::spawn_blocking(move || request.query_instance())
            })
            .collect::<Vec<_>>();

        // a bug is preventing us from using stream::iter.
        // for some reason, if the config does not provide instances for mssql i.e. a discovery of the instances happens
        //   then we cannot await the iterator. the plugin will hang.
        //   the cause remains unknown for now. might be a bug in tokio, futures or mssql-browser?.
        //   so for now, we poll the futures manually
        // also, remembeer timeouts
        let timeout = (*(config.timeout.as_ref().unwrap_or(&20))) as u64;
        let start = Instant::now();
        while !handles.iter().all(|h| h.is_finished()) {
            if start.elapsed().as_secs() > timeout {
                for h in handles {
                    h.abort();
                }
                return Err(Error::Timeout(timeout));
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let data = handles
            .into_iter()
            .map(|h| h.now_or_never().unwrap())
            .collect::<Vec<_>>();

        // let data = stream::iter(handles)
        //    .buffer_unordered(config.instances.len())
        //    .collect::<Vec<_>>()
        //    .await;
        debug!("All queries completed");
        if let Err(e) = sql_plugin.save_counters().await {
            warn!("failed to save counters: {e}");
        }

        let mut query_result: DataMap = HashMap::with_capacity(query.len());
        for instance_result in data {
            match instance_result.unwrap() {
                Ok(datatables) => {
                    for (dt_id, data_result) in datatables {
                        if let Ok(anno) = query_result
                            .entry(dt_id)
                            .or_insert(Ok(Annotated {
                                value: Vec::new(),
                                warnings: Vec::new(),
                            }))
                            .as_mut()
                        {
                            match data_result {
                                Ok(table) => anno.value.extend(table),
                                Err(e) => anno.warnings.push(Warning {
                                    verbosity: Verbosity::Warning,
                                    message: e,
                                }),
                            }
                        }
                    }
                }
                Err(e) => {
                    for dt_id in query.keys().cloned() {
                        if let Ok(anno) = query_result
                            .entry(dt_id)
                            .or_insert(Ok(Annotated {
                                value: Vec::new(),
                                warnings: Vec::new(),
                            }))
                            .as_mut()
                        {
                            anno.warnings.push(Warning {
                                verbosity: Verbosity::Warning,
                                message: DTWarning::Error(e.to_string()),
                            })
                        }
                    }
                }
            }
        }

        Ok(query_result)
    }
}
