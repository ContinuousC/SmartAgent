/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::env;
use std::fmt::Write;
use std::path::PathBuf;

use async_trait::async_trait;
use netsnmp::Oid;

use agent_utils::{KeyVault, TryGetFrom};
use etc_base::{ProtoDataFieldId, ProtoDataTableId, ProtoQueryMap};
use parking_lot::Mutex;
use protocol::{DataFieldSpec, DataTableSpec};
//use etc::{...};
//use vault::VaultSock;

use super::config::Config;
use super::counters::Counters;
use super::error::{DTError, DTWarning, Error, Result, TypeError, TypeResult};
use super::get::Gets;
use super::index::Index;
use super::input::{Input, ObjectId};
use super::query::{self, DataMap, WalkMap};
use super::stats::Stats;
use super::walk::Walks;

pub struct Plugin {
    cache_dir: PathBuf,
    snmp: netsnmp::NetSNMP,
    key_vault: KeyVault,
}

#[async_trait]
impl protocol::LocalPlugin for Plugin {
    type Error = Error;
    type TypeError = TypeError;
    type DTError = DTError;
    type DTWarning = DTWarning;

    type Input = Input;
    type Config = Config;

    const PROTOCOL: &'static str = "SNMP";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    fn show_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> Result<String> {
        let mut out = String::new();

        for (table_oid, field_oids) in self.get_queries(query, input)? {
            writeln!(
                out,
                "SNMP: {}: {}",
                table_oid,
                field_oids
                    .iter()
                    .map(|oid| oid.in_table(&table_oid).to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )
            .unwrap();
        }

        Ok(out)
    }

    async fn run_queries(
        &self,
        input: &Input,
        config: &Config,
        query: &ProtoQueryMap,
    ) -> Result<DataMap> {
        let host_cache_dir = self.cache_dir.join(&config.host_name);
        let stats_file = host_cache_dir.join("snmp_table_length.json");
        let counters_file = host_cache_dir.join("snmp_counters.json");

        let stats = Mutex::new(Stats::load(&stats_file).await?);
        let mut counters = Counters::load(&counters_file).await?;

        let result = self
            .retrieve_data(input, config, query, &stats, &mut counters)
            .await;
        if let Err(e) = &result {
            log::debug!("SNMP: retrieve_data failed: {e}");
        }

        stats.into_inner().save(&stats_file).await?;
        counters.save(&counters_file).await?;

        result
    }

    fn get_tables(
        &self,
        input: &Self::Input,
    ) -> TypeResult<HashMap<ProtoDataTableId, DataTableSpec>> {
        let mut table_fields = HashMap::new();
        for (obj_id, field) in &input.scalars {
            table_fields
                .entry(field.table.clone())
                .or_insert_with(HashSet::new)
                .insert(obj_id.to_field_id(input)?);
        }

        let mut tables = HashMap::new();
        for (obj_id, fields) in table_fields {
            match obj_id {
                None => {
                    tables.insert(
                        ProtoDataTableId(String::from("noIndex")),
                        DataTableSpec {
                            name: String::from("noIndex"),
                            singleton: true,
                            keys: HashSet::new(),
                            fields,
                        },
                    );
                }
                Some(obj_id) => {
                    let object = obj_id.try_get_from(&input.objects)?;
                    let entry = obj_id.try_get_from(&input.tables)?;
                    tables.insert(
                        obj_id.to_table_id(input)?,
                        DataTableSpec {
                            name: object.name.to_string(),
                            singleton: false,
                            keys: entry
                                .get_index(input)?
                                .to_field_id_set(input)?,
                            fields: &fields
                                | &entry
                                    .get_index(input)?
                                    .to_field_id_set(input)?,
                        },
                    );
                }
            }
        }
        Ok(tables)
    }

    fn get_fields(
        &self,
        input: &Self::Input,
    ) -> TypeResult<HashMap<ProtoDataFieldId, DataFieldSpec>> {
        let mut fields = HashMap::new();
        for (obj_id, field) in &input.scalars {
            let object = obj_id.try_get_from(&input.objects)?;
            fields.insert(
                obj_id.to_field_id(input)?,
                DataFieldSpec {
                    name: object.name.to_string(),
                    input_type: field.get_type()?,
                },
            );
        }
        Ok(fields)
    }

    /*fn get_field_type(
        &self,
        field_id: DataFieldId,
        input: &Self::Input,
    ) -> TypeResult<Type> {
        field_id
            .try_get_from(&input.data_fields)?
            .try_get_from(&input.scalars)?
            .get_type()?
    }*/
}

impl Plugin {
    pub fn new(cache_dir: PathBuf, key_vault: KeyVault) -> Self {
        Self {
            cache_dir,
            snmp: netsnmp::init("SmartM SNMP Agent"),
            key_vault,
        }
    }

    fn get_queries(
        &self,
        query: &ProtoQueryMap,
        input: &Input,
    ) -> Result<Vec<(Oid, Vec<Oid>)>> {
        let mut queries = HashMap::new();

        for (table_id, field_ids) in query {
            let (table_oid, index) =
                match ObjectId::from_table_id(table_id, input)? {
                    Some(obj_id) => (
                        obj_id.try_get_from(&input.objects)?.oid.clone(),
                        obj_id.try_get_from(&input.tables)?.get_index(input)?,
                    ),
                    None => (Oid::empty(), Index::empty()),
                };

            let field_oids =
                queries.entry(table_oid).or_insert_with(HashSet::new);

            for field_id in field_ids {
                let obj_id = ObjectId::from_field_id(field_id, input)?;
                if !index.contains(&obj_id) {
                    field_oids.insert(
                        obj_id.try_get_from(&input.objects)?.oid.clone(),
                    );
                }
            }
        }

        let mut query_list = queries
            .into_iter()
            .map(|(table_oid, field_oids)| {
                let mut field_oid_list =
                    field_oids.into_iter().collect::<Vec<Oid>>();
                field_oid_list.sort();
                (table_oid, field_oid_list)
            })
            .collect::<Vec<(Oid, Vec<Oid>)>>();
        query_list.sort();

        Ok(query_list)
    }

    async fn get_auth_from_vault(
        &self,
        mut auth: netsnmp::Auth,
    ) -> Result<netsnmp::Auth> {
        match &mut auth {
            netsnmp::Auth::V1(params) => {
                params.community = self
                    .key_vault
                    .retrieve_password(params.community.to_string())
                    .await?;
            }
            netsnmp::Auth::V2c(params) => {
                params.community = self
                    .key_vault
                    .retrieve_password(params.community.to_string())
                    .await?;
            }
            netsnmp::Auth::V3(auth) => match &mut auth.level {
                netsnmp::V3Level::NoAuthNoPriv => {}
                netsnmp::V3Level::AuthNoPriv(netsnmp::V3AuthNoPriv {
                    auth,
                }) => {
                    auth.password = self
                        .key_vault
                        .retrieve_password(auth.password.to_string())
                        .await?;
                }
                netsnmp::V3Level::AuthPriv(netsnmp::V3AuthPriv {
                    auth,
                    privacy,
                }) => {
                    auth.password = self
                        .key_vault
                        .retrieve_password(auth.password.to_string())
                        .await?;
                    privacy.password = self
                        .key_vault
                        .retrieve_password(privacy.password.to_string())
                        .await?;
                }
            },
        }
        Ok(auth)
    }

    async fn retrieve_data(
        &self,
        input: &Input,
        config: &Config,
        query_map: &ProtoQueryMap,
        stats: &Mutex<Stats>,
        counters: &mut Counters,
    ) -> Result<DataMap> {
        let queries =
            query::get_queries(input, config, query_map, &mut stats.lock())?;
        let data = self.get_raw_table(config, queries, stats).await?;
        query::build_tables(input, query_map, data, counters)
    }

    pub async fn get_raw_table(
        &self,
        config: &Config,
        queries: HashMap<String, (Walks, Gets)>,
        stats: &Mutex<Stats>,
    ) -> Result<WalkMap> {
        match config.host_config.use_walk {
            true => query::retrieve_data_from_walk(queries, stats).await,
            false => {
                // let auth = config.host_config.auth.clone()
                /*match config.auth.clone() {
                    Some(auth) =>  match &None /* vault_sock */ {
                    Some(sock) => Some(self.get_auth_from_vault(
                        &mut *sock.lock().await, auth, ctx).await?),
                    None => Some(auth)
                    },
                    None => None
                }*/
                let auth = match self.key_vault {
                    KeyVault::Identity => config.host_config.auth.clone(),
                    _ => {
                        if let Some(auth) = config.host_config.auth.clone() {
                            Some(self.get_auth_from_vault(auth).await?)
                        } else {
                            None
                        }
                    }
                };
                match config.host_config.bulk_host {
                    true => {
                        query::retrieve_data_bulk(
                            &self.snmp, &auth, config, queries, stats,
                        )
                        .await
                    }
                    false => {
                        query::retrieve_data_nobulk(
                            &self.snmp, &auth, config, queries, stats,
                        )
                        .await
                    }
                }
            }
        }
    }
}
