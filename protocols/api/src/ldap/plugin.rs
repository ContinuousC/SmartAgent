/******************************************************************************
 * Copyright ContinuousC. Licensed under the "Elastic License 2.0".           *
 ******************************************************************************/

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use futures::{stream, StreamExt};
use ldap3::{Ldap, SearchEntry};
use log::{debug, error, info, trace, warn};

use agent_utils::{KeyVault, TryGetFrom};
use etc_base::{
    Annotated, ProtoDataFieldId, ProtoDataTableId, ProtoQueryMap, Warning,
};
use logger::Verbosity;
use protocol::CounterDb;
use regex::Regex;
use value::{DataError, Value};

use super::config::{LdapScope, ReplicationConfig, SearchConfig};
use crate::error::Result as APIResult;
use crate::input::ParameterType;
use crate::{
    error::{DTError, DTWarning},
    input::{FieldSpec, TableSpec},
    ldap::{Config, Error, Result},
    plugin::{DataMap, TableData},
    APIPlugin, Input, Plugin as ProtPlugin,
};

lazy_static::lazy_static! {
    static ref REPLICATION_UPDATE_STATUS_REGEX: Regex = Regex::new(r"Error \((\d)\) (.*)").unwrap();
}

#[derive(serde::Deserialize, Debug)]
struct ReplicationStatus {
    state: String,
    ldap_rc: String, // should be i64, but is returned as string in the json
    ldap_rc_text: String,
    repl_rc: String, // should be i64, but is returned as string in the json
    repl_rc_text: String,
    date: DateTime<Utc>,
    message: String,
}

pub struct Plugin {
    key_vault: KeyVault,
    config: Vec<Config>,
    counterdb: PathBuf,
}

impl Plugin {
    pub async fn new(
        key_vault: KeyVault,
        config: Vec<Config>,
        counterdb: PathBuf,
    ) -> Result<Self> {
        Ok(Self {
            key_vault,
            config,
            counterdb,
        })
    }

    async fn check_ldaps(
        &self,
        ldap: &mut Ldap,
        dfs: &HashMap<ProtoDataFieldId, &FieldSpec>,
        config: &Config,
        service: &String,
    ) -> TableData {
        let mut value = Vec::with_capacity(config.search_config.len());
        let mut warnings = Vec::with_capacity(config.search_config.len());

        for sc in &config.search_config {
            info!("Searching '{}' for {}", sc.base_dn, service);
            match sc.timed_search(ldap, config.host_config.timeout()).await {
                Err(e) => warnings.push(Warning {
                    message: e.for_service(service).to_dtwarning(),
                    verbosity: Verbosity::Warning,
                }),
                Ok((time, entries)) => value.push(
                    dfs.iter()
                        .map(|(df_id, df)| {
                            (
                                df_id.clone(),
                                match df.parameter_name.as_str() {
                                    "service" => Ok(Value::BinaryString(
                                        service.as_bytes().to_vec(),
                                    )),
                                    "base_dn" => Ok(Value::BinaryString(
                                        sc.base_dn.as_bytes().to_vec(),
                                    )),
                                    "scope" => Ok(Value::BinaryString(
                                        sc.scope
                                            .to_string()
                                            .as_bytes()
                                            .to_vec(),
                                    )),
                                    "filter" => Ok(Value::BinaryString(
                                        sc.filter.as_bytes().to_vec(),
                                    )),
                                    "attributes" => Ok(Value::BinaryString(
                                        sc.attributes
                                            .join("<br>")
                                            .as_bytes()
                                            .to_vec(),
                                    )),
                                    "searchtime" => {
                                        Ok(Value::Integer(time as i64))
                                    }
                                    "num_entries" => {
                                        Ok(Value::Integer(entries.len() as i64))
                                    }
                                    _ => Err(DataError::Missing),
                                },
                            )
                        })
                        .collect(),
                ),
            }
        }

        Ok(Annotated { value, warnings })
    }

    async fn replication(
        &self,
        ldap: &mut Ldap,
        dfs: &HashMap<ProtoDataFieldId, &FieldSpec>,
        config: &Config,
        service: &String,
        counterdb: &mut Arc<Mutex<CounterDb>>,
    ) -> TableData {
        let replications_dns = config
            .replication_config
            .as_ref()
            .unwrap_or(&ReplicationConfig::All)
            .get_replication_dns();
        let mut value = Vec::with_capacity(replications_dns.len());
        let mut warnings = Vec::with_capacity(replications_dns.len());

        for replication_dn in replications_dns {
            info!(
                "Checking replication_dn '{}' for {}",
                &replication_dn, service
            );
            let search_result = SearchConfig::new(
                replication_dn,
                if config.replication_config == Some(ReplicationConfig::All) {
                    LdapScope::Subtree
                } else {
                    LdapScope::Base
                },
                "(objectclass=nsDS5ReplicationAgreement)",
                dfs.values()
                    .map(|df| &df.parameter_name)
                    .chain([
                        &String::from("nsds5replicaLastUpdateStatus"),
                        &String::from("nsds5replicaLastUpdateStatusJSON"),
                    ])
                    .collect::<Vec<_>>(),
            )
            .search(ldap)
            .await;

            let result_entries = match search_result {
                Err(e) => {
                    warnings.push(Warning {
                        message: e.for_service(service).to_dtwarning(),
                        verbosity: Verbosity::Warning,
                    });
                    continue;
                }
                Ok(result_entries) => result_entries,
            };
            debug!(
                "found {} replication entries for {service}",
                result_entries.len()
            );

            for result_entry in result_entries {
                let search_entry = SearchEntry::construct(result_entry);
                trace!("replication entry for {service}: {search_entry:#?}");
                let repl_status: Option<ReplicationStatus> = match search_entry
                    .attrs
                    .get("nsds5replicaLastUpdateStatusJSON")
                    .map(|attr| serde_json::from_str(&attr[0]))
                {
                    None => {
                        warnings.push(Warning {
                            message: Error::AttributeNotFound(
                                "nsds5replicaLastUpdateStatusJSON".to_string(),
                                search_entry.dn.clone(),
                            )
                            .for_service(service)
                            .to_dtwarning(),
                            verbosity: Verbosity::Warning,
                        });
                        None
                    }
                    Some(Err(e)) => {
                        warnings.push(Warning {
                            message: Error::UnexpectedReplicationStatusJSON(
                                search_entry.dn.clone(),
                                e,
                            )
                            .for_service(service)
                            .to_dtwarning(),
                            verbosity: Verbosity::Warning,
                        });
                        None
                    }
                    Some(Ok(status)) => Some(status),
                };
                let repl_caps = match search_entry
                    .attrs
                    .get("nsds5replicaLastUpdateStatus")
                    .map(|attr| {
                        REPLICATION_UPDATE_STATUS_REGEX.captures(&attr[0])
                    }) {
                    None => {
                        warnings.push(Warning {
                            message: Error::AttributeNotFound(
                                "nsds5replicaLastUpdateStatus".to_string(),
                                search_entry.dn.clone(),
                            )
                            .for_service(service)
                            .to_dtwarning(),
                            verbosity: Verbosity::Warning,
                        });
                        None
                    }
                    Some(None) => {
                        warnings.push(Warning {
                            message: Error::UnexpectedReplicationStatus(
                                search_entry.dn.clone(),
                            )
                            .for_service(service)
                            .to_dtwarning(),
                            verbosity: Verbosity::Warning,
                        });
                        None
                    }
                    Some(Some(m)) => Some(m),
                };
                trace!("replication status: {:?}", &repl_status);
                let now = SystemTime::now();

                value.push(
                    dfs.iter()
                        .map(|(df_id, df)| {
                            (
                                df_id.clone(),
                                match df.parameter_name.as_str() {
                                    "service" => Ok(Value::BinaryString(
                                        service.as_bytes().to_vec(),
                                    )),
                                    "replica_agreement" => {
                                        Ok(Value::BinaryString(
                                            search_entry.dn.as_bytes().to_vec(),
                                        ))
                                    }

                                    // result from regex
                                    "status_code" => repl_caps
                                        .as_ref()
                                        .and_then(|caps| caps.get(1))
                                        .ok_or(DataError::Missing)
                                        // we can unwrap due to the regex check
                                        .map(|m| {
                                            Value::Integer(
                                                m.as_str()
                                                    .parse::<i64>()
                                                    .unwrap(),
                                            )
                                        }),
                                    "status" => repl_caps
                                        .as_ref()
                                        .and_then(|caps| caps.get(2))
                                        .ok_or(DataError::Missing)
                                        .map(|m| {
                                            Value::BinaryString(
                                                m.as_str().as_bytes().to_vec(),
                                            )
                                        }),

                                    // result from replication state
                                    "state" => repl_status
                                        .as_ref()
                                        .map(|status| {
                                            Value::BinaryString(
                                                status
                                                    .state
                                                    .as_bytes()
                                                    .to_vec(),
                                            )
                                        })
                                        .ok_or(DataError::Missing),
                                    "ldap_rc" => repl_status
                                        .as_ref()
                                        .ok_or(DataError::Missing)
                                        .and_then(|status| {
                                            status.ldap_rc.parse()
                                                .map_err(|e| DataError::TypeError(format!("Could not parse String to I64: {e}")))
                                                .map(Value::Integer)
                                        }),
                                    "ldap_rc_text" => repl_status
                                        .as_ref()
                                        .map(|status| {
                                            Value::BinaryString(
                                                status
                                                    .ldap_rc_text
                                                    .as_bytes()
                                                    .to_vec(),
                                            )
                                        })
                                        .ok_or(DataError::Missing),
                                    "repl_rc" => repl_status
                                        .as_ref()
                                        .ok_or(DataError::Missing)
                                        .and_then(|status| {
                                            status.repl_rc.parse()
                                                .map_err(|e| DataError::TypeError(format!("Could not parse String to I64: {e}")))
                                                .map(Value::Integer)
                                        }),
                                    "repl_rc_text" => repl_status
                                        .as_ref()
                                        .map(|status| {
                                            Value::BinaryString(
                                                status
                                                    .repl_rc_text
                                                    .as_bytes()
                                                    .to_vec(),
                                            )
                                        })
                                        .ok_or(DataError::Missing),
                                    "date" => repl_status
                                        .as_ref()
                                        .map(|status| Value::Time(status.date))
                                        .ok_or(DataError::Missing),
                                    "message" => repl_status
                                        .as_ref()
                                        .map(|status| {
                                            Value::BinaryString(
                                                status
                                                    .message
                                                    .as_bytes()
                                                    .to_vec(),
                                            )
                                        })
                                        .ok_or(DataError::Missing),

                                    _ => self.parse_attr(
                                        df,
                                        &search_entry,
                                        service,
                                        now,
                                        counterdb,
                                    ),
                                },
                            )
                        })
                        .collect(),
                );
            }
        }

        Ok(Annotated { value, warnings })
    }

    async fn search(
        &self,
        ldap: &mut Ldap,
        dfs: &HashMap<ProtoDataFieldId, &FieldSpec>,
        dn: impl ToString,
        scope: LdapScope,
        service: &String,
        counterdb: &mut Arc<Mutex<CounterDb>>,
    ) -> TableData {
        let mut value = Vec::new();
        let mut warnings = Vec::new();
        let dn = dn.to_string();

        info!("Searching dn {} for {}", &dn, service);
        let search_result = SearchConfig::new(
            dn.clone(),
            scope,
            "(objectclass=*)",
            dfs.values().map(|df| &df.parameter_name).collect(),
        )
        .search(ldap)
        .await;
        debug!(
            "Result of {} for {}: {:?}",
            &dn,
            service,
            search_result.as_ref().map(|res| res.len())
        );
        let now = SystemTime::now();

        match search_result {
            Err(e) => warnings.push(Warning {
                message: e.for_service(service).to_dtwarning(),
                verbosity: Verbosity::Warning,
            }),
            Ok(result_entries) => {
                for result_entry in result_entries {
                    let search_entry = SearchEntry::construct(result_entry);
                    trace!(
                        "Instance in {} for {}: {:#?}",
                        &dn,
                        service,
                        &search_entry
                    );
                    value.push(
                        dfs.iter()
                            .map(|(df_id, df)| {
                                (
                                    df_id.clone(),
                                    self.parse_attr(
                                        df,
                                        &search_entry,
                                        service,
                                        now,
                                        counterdb,
                                    ),
                                )
                            })
                            .collect(),
                    );
                }
            }
        };

        Ok(Annotated { value, warnings })
    }

    /*
    // To be done with os checks
    async fn disk_space(&self, ldap: &mut Ldap, dfs: &HashMap<ProtoDataFieldId, &FieldSpec>, service: &String) -> TableData {
        let mut value = Vec::new();
        let mut warnings = Vec::new();


        Ok(Annotated { value, warnings })
    }
    */

    fn parse_attr(
        &self,
        df: &FieldSpec,
        se: &SearchEntry,
        service: &String,
        time: SystemTime,
        counterdb: &mut Arc<Mutex<CounterDb>>,
    ) -> std::result::Result<Value, DataError> {
        // trace!("parsing {:?}", df);
        match df.parameter_name.as_str() {
            "service" => Ok(Value::BinaryString(service.as_bytes().to_vec())),
            "dn" => Ok(Value::BinaryString(se.dn.as_bytes().to_vec())),
            param => se
                .attrs
                .get(param)
                .map(|val| match df.parameter_type {
                    ParameterType::String => {
                        Ok(Value::BinaryString(val.join("<br>").into_bytes()))
                    }
                    ParameterType::Integer => val
                        .get(0)
                        .ok_or(DataError::Missing)
                        .map(|i| {
                            i.parse::<i64>()
                                .map_err(|e| {
                                    DataError::TypeError(e.to_string())
                                })
                                .map(Value::Integer)
                        })
                        .and_then(std::convert::identity),
                    ParameterType::Time => val
                        .get(0)
                        .ok_or(DataError::Missing)
                        .map(|dt| {
                            DateTime::parse_from_str(dt, "%Y%m%d%H%M%SZ")
                                .map_err(|e| {
                                    DataError::TypeError(format!(
                                        "Cannot parse '{}' to a datetime: {}",
                                        dt, e
                                    ))
                                })
                                .map(|dt| Value::Time(dt.with_timezone(&Utc)))
                        })
                        .and_then(std::convert::identity),
                    ParameterType::Boolean => val
                        .get(0)
                        .ok_or(DataError::Missing)
                        .map(|b| match b.as_str() {
                            "TRUE" => Ok(Value::Boolean(true)),
                            "FALSE" => Ok(Value::Boolean(false)),
                            b => Err(DataError::TypeError(format!(
                                "{} is not a boolean",
                                b
                            ))),
                        })
                        .and_then(std::convert::identity),
                    ParameterType::Difference => val
                        .get(0)
                        .ok_or(DataError::Missing)
                        .map(|val| {
                            val.parse::<u64>()
                                .map_err(|e| {
                                    DataError::TypeError(e.to_string())
                                })
                                .map(|i| {
                                    counterdb.lock().unwrap().difference(
                                        format!(
                                            "{}.{}.{}",
                                            service, se.dn, param
                                        ),
                                        i,
                                        time,
                                    )
                                })
                                .and_then(std::convert::identity)
                        })
                        .and_then(std::convert::identity),
                    ParameterType::Counter => val
                        .get(0)
                        .ok_or(DataError::Missing)
                        .map(|val| {
                            val.parse::<u64>()
                                .map_err(|e| {
                                    DataError::TypeError(e.to_string())
                                })
                                .map(|i| {
                                    counterdb.lock().unwrap().counter(
                                        format!(
                                            "{}.{}.{}",
                                            service, se.dn, param
                                        ),
                                        i,
                                        time,
                                    )
                                })
                                .and_then(std::convert::identity)
                        })
                        .and_then(std::convert::identity),

                    s => Err(DataError::TypeError(format!(
                        "Uknown parametertype: {}",
                        s
                    ))),
                })
                .ok_or(DataError::Missing)
                .and_then(std::convert::identity),
        }
    }

    async fn request<'a>(
        &self,
        config: &Config,
        ldap_queries: &LdapQuery<'a>,
        mut counterdb: Arc<Mutex<CounterDb>>,
    ) -> Result<HashMap<ProtoDataTableId, TableData>> {
        let service = config
            .service_name
            .as_ref()
            .unwrap_or(&config.host_config.get_url())
            .to_string();

        debug!("Connecting to service: {}", &service);
        let (conn, mut ldap) = config
            .host_config
            .connect()
            .await
            .map_err(|e| e.for_service(&service))?;
        ldap3::drive!(conn);

        if let Some(bind_config) = config.bind_config.as_ref() {
            debug!("binding to service: {}", &service);
            bind_config
                .bind(&mut ldap, &self.key_vault)
                .await
                .map_err(|e| e.for_service(&service))?;
        }

        let mut responses = HashMap::with_capacity(ldap_queries.len());
        for ((dt_id, dt), dfs) in ldap_queries.iter() {
            debug!("schedule request {} for {}", &dt_id, &service);
            // resets after every request
            ldap.with_timeout(config.host_config.timeout());
            responses.insert(
                dt_id.clone(),
                match dt.command_name.as_str() {
                    // specific checks
                    "check_ldap" => {
                        self.check_ldaps(&mut ldap, dfs, config, &service).await
                    }
                    "replication" => {
                        self.replication(
                            &mut ldap,
                            dfs,
                            config,
                            &service,
                            &mut counterdb,
                        )
                        .await
                    }
                    // "disk_space" => self.disk_space(&mut ldap, dfs, &service).await, -> check on os level

                    // more general checks
                    "search" => {
                        self.search(
                            &mut ldap,
                            dfs,
                            &dt.command_line,
                            LdapScope::Subtree,
                            &service,
                            &mut counterdb,
                        )
                        .await
                    }
                    "specific_search" => {
                        self.search(
                            &mut ldap,
                            dfs,
                            &dt.command_line,
                            LdapScope::Base,
                            &service,
                            &mut counterdb,
                        )
                        .await
                    }
                    "monitor" => {
                        self.search(
                            &mut ldap,
                            dfs,
                            "cn=monitor",
                            LdapScope::Base,
                            &service,
                            &mut counterdb,
                        )
                        .await
                    }
                    "snmp" => {
                        self.search(
                            &mut ldap,
                            dfs,
                            "cn=snmp,cn=monitor",
                            LdapScope::Base,
                            &service,
                            &mut counterdb,
                        )
                        .await
                    }
                    _ => Err(DTError::CommandNotFound(dt.command_name.clone())),
                },
            );
        }

        for (dt_id, dt_res) in &responses {
            match dt_res {
                Err(e) => error!(
                    "Error while retrieving datatable {}: {:?}",
                    dt_id, e
                ),
                Ok(ano) => {
                    for w in &ano.warnings {
                        warn!(
                            "Warning while retrieving datatable {}: {:?}",
                            dt_id, w
                        );
                    }
                }
            }
        }

        ldap.unbind()
            .await
            .map_err(Error::LdapUnBind)
            .map_err(|e| e.for_service(&service))?;
        Ok(responses)
    }
}

type LdapQuery<'a> = HashMap<
    (ProtoDataTableId, &'a TableSpec),
    HashMap<ProtoDataFieldId, &'a FieldSpec>,
>;

#[async_trait::async_trait]
impl APIPlugin for Plugin {
    async fn run_queries(
        &self,
        input: &Input,
        query: &ProtoQueryMap,
    ) -> APIResult<DataMap> {
        info!("Using LDAP API plugin");

        let counterdb = Arc::new(Mutex::new(
            CounterDb::load(self.counterdb.join("counters.json")).await?,
        ));

        let ldap_queries = query
            .iter()
            .map(|(dt_id, df_ids)| {
                let command = ProtPlugin::get_datatable_id(dt_id)
                    .try_get_from(&input.data_tables)?;
                Ok((
                    (dt_id.clone(), command),
                    df_ids
                        .iter()
                        .map(|df_id| {
                            Ok((
                                df_id.clone(),
                                ProtPlugin::get_datafield_id(df_id)
                                    .try_get_from(&input.data_fields)?,
                            ))
                        })
                        .collect::<Result<_>>()?,
                ))
            })
            .collect::<Result<LdapQuery>>()?;
        let dts = ldap_queries
            .iter()
            .map(|((dt_id, _), _)| dt_id)
            .cloned()
            .collect::<HashSet<_>>();

        info!("Scheduling {} services", self.config.len());
        let responses: Vec<Result<HashMap<ProtoDataTableId, TableData>>> =
            stream::iter(
                self.config
                    .iter()
                    .map(|c| self.request(c, &ldap_queries, counterdb.clone()))
                    .collect::<Vec<_>>(),
            )
            .buffer_unordered(8)
            .collect()
            .await;
        // we can unwrap the arc & mutex since there should be no more pointers after the requests
        Arc::try_unwrap(counterdb)
            .unwrap()
            .into_inner()
            .unwrap()
            .save()
            .await?;

        Ok(responses
            .into_iter()
            .fold(HashMap::new(), |mut accum, elem| {
                match elem {
                    Err(e) => {
                        error!(
                            "Encounterd an error while retrieving data: {:?}",
                            e
                        );
                        for dt_id in dts.iter() {
                            let accum_res =
                                accum.remove(dt_id).unwrap_or_else(|| {
                                    Ok(Annotated {
                                        value: Vec::new(),
                                        warnings: Vec::new(),
                                    })
                                });
                            accum.insert(
                                dt_id.clone(),
                                accum_res.map(|accum_ano| Annotated {
                                    value: accum_ano.value,
                                    warnings: accum_ano
                                        .warnings
                                        .into_iter()
                                        .chain(
                                            vec![Warning {
                                                message: DTWarning::Ldap(
                                                    Error::Custom(
                                                        e.to_string(),
                                                    ),
                                                ),
                                                verbosity: Verbosity::Warning,
                                            }]
                                            .into_iter(),
                                        )
                                        .collect::<Vec<_>>(),
                                }),
                            );
                        }
                    }
                    Ok(elem) => {
                        for (dt_id, elem_res) in elem.into_iter() {
                            let accum_res =
                                accum.remove(&dt_id).unwrap_or_else(|| {
                                    Ok(Annotated {
                                        value: Vec::new(),
                                        warnings: Vec::new(),
                                    })
                                });
                            match (accum_res, elem_res) {
                                (Err(accum_err), Err(_)) => {
                                    accum.insert(dt_id, Err(accum_err))
                                }
                                (Ok(_), Err(elem_err)) => {
                                    accum.insert(dt_id, Err(elem_err))
                                }
                                (Err(accum_err), Ok(_)) => {
                                    accum.insert(dt_id, Err(accum_err))
                                }
                                (Ok(accum_ano), Ok(elem_ano)) => accum.insert(
                                    dt_id,
                                    Ok(Annotated {
                                        value: accum_ano
                                            .value
                                            .into_iter()
                                            .chain(elem_ano.value.into_iter())
                                            .collect::<Vec<_>>(),
                                        warnings: accum_ano
                                            .warnings
                                            .into_iter()
                                            .chain(
                                                elem_ano.warnings.into_iter(),
                                            )
                                            .collect::<Vec<_>>(),
                                    }),
                                ),
                            };
                        }
                    }
                };
                accum
            }))
    }
}
